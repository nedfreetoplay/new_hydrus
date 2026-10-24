//! Rust port of the FFMPEG parsing half of
//! `hydrus/core/files/HydrusVideoHandling.py`.
//!
//! The renderer at the end of the Python module is deliberately not part of
//! this file. Rendering is provided by [`crate::files::hydrus_ffmpeg_rendering`], and
//! keeping the ffprobe-output parsing here means the file handling layer can
//! ask for MIME type and properties without depending on a renderer object.

use std::path::Path;

use image::{DynamicImage, RgbImage, RgbaImage};
use regex::Regex;

use crate::files::hydrus_audio_handling;
use crate::files::hydrus_ffmpeg;
use crate::files::hydrus_ffmpeg_parsing;
use crate::hydrus_constants::{
    ANIMATIONS, MIMES_THAT_WE_CAN_CHECK_FOR_TRANSPARENCY, Mime, running_from_source,
};
use crate::hydrus_data;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_time;
use crate::processes::hydrus_subprocess::{SubprocessCommand, SubprocessContext};

fn damaged(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DamagedOrUnusualFile, message)
}

/// `file_is_animated( path )`
///
/// This is only used for JXL in Python. It renders properties and checks the
/// frame count.
pub fn file_is_animated(path: impl AsRef<Path>) -> bool {
    get_ffmpeg_video_properties(path, false)
        .map(|(_, _, num_frames, _)| num_frames > 1)
        .unwrap_or(false)
}

/// `get_ffmpeg_video_properties( path, force_count_frames_manually = False )`
pub fn get_ffmpeg_video_properties(
    path: impl AsRef<Path>,
    force_count_frames_manually: bool,
) -> HydrusResult<((u32, u32), i64, u32, bool)> {
    let path = path.as_ref();

    let ffmpeg_lines = hydrus_ffmpeg::get_ffmpeg_info_lines(path, None)?;

    let (has_video, _, video_stream_mapping) = parse_ffmpeg_video_format(&ffmpeg_lines);

    if !has_video {
        return Err(damaged(
            "Wanted to parse video data, but file did not appear to have a video stream!",
        ));
    }

    let resolution = parse_ffmpeg_video_resolution(&ffmpeg_lines, false)?;

    let (file_duration_in_s, stream_duration_in_s) = parse_ffmpeg_duration(&ffmpeg_lines)?;

    let mut duration_s = stream_duration_in_s;

    let (mut fps, mut confident_fps) = parse_ffmpeg_fps(&ffmpeg_lines, false)?;

    if duration_s.is_none() && !confident_fps {
        // Default to fall back on.
        (fps, confident_fps) = (24.0, true);
    }

    if fps == 0.0 {
        fps = 1.0;
    }

    let mut force_count_frames_manually = force_count_frames_manually;

    if duration_s.is_none() {
        force_count_frames_manually = true;
    } else if let Some(duration) = duration_s {
        // If the file is big or long, don't try to force a manual count when
        // one was not explicitly requested. We don't care about a dropped frame
        // on a 10 minute video.
        let num_frames_seems_ok_to_count = duration < 30.0;
        let file_is_ok_size = std::fs::metadata(path)
            .map(|metadata| metadata.len() < 256 * 1024 * 1024)
            .unwrap_or(false);

        if num_frames_seems_ok_to_count && file_is_ok_size {
            let last_frame_has_unusual_duration = (duration * fps).fract().abs() > 0.0;
            let unusual_video_start = file_duration_in_s != stream_duration_in_s;

            if !confident_fps || last_frame_has_unusual_duration || unusual_video_start {
                force_count_frames_manually = true;
            }
        }
    }

    let num_frames;

    if force_count_frames_manually {
        let count_frames_manually_lines =
            hydrus_ffmpeg::get_ffmpeg_info_lines(path, Some(video_stream_mapping.as_str()))?;

        num_frames = parse_ffmpeg_num_frames_manually(&count_frames_manually_lines)?;

        if num_frames > 0 && duration_s.is_some() {
            let duration = duration_s.unwrap_or_default();
            let implied_fps_given_what_we_know = f64::from(num_frames) / duration;

            if fps > 0.0 && fps < 1000.0 && implied_fps_given_what_we_know > 1000.0 {
                // FFMPEG occasionally reports a 40 ms duration for a file with
                // hundreds of frames. Trust the "not confident" fps in that
                // case.
                duration_s = None;
            }
        }

        if duration_s.is_none() {
            duration_s = Some(f64::from(num_frames) / fps);
        }
    } else {
        let duration = duration_s.unwrap_or(1.0);

        num_frames = (duration * fps) as u32;
    }

    let duration_in_ms = hydrus_time::millisecondise_s(duration_s).unwrap_or(0);

    let has_audio = video_has_audio(path, &ffmpeg_lines)?;

    Ok((resolution, duration_in_ms, num_frames, has_audio))
}

/// `has_video_stream( path )`
pub fn has_video_stream(path: impl AsRef<Path>) -> HydrusResult<bool> {
    let lines = hydrus_ffmpeg::get_ffmpeg_info_lines(path, None)?;

    Ok(parse_ffmpeg_has_video(&lines))
}

/// `parse_ffmpeg_duration( lines )`
pub fn parse_ffmpeg_duration(lines: &[String]) -> HydrusResult<(Option<f64>, Option<f64>)> {
    let Some(line) = lines
        .iter()
        .find(|line| line.trim_start().starts_with("Duration:"))
    else {
        return Err(damaged("Error reading duration!"));
    };

    if line.contains("Duration: N/A") {
        return Ok((None, None));
    }

    let start_offset = if line.contains("start:") {
        let start_regex =
            Regex::new(r"start: (-?[0-9]+\.[0-9]*)").expect("the duration start regex is valid");

        let Some(captures) = start_regex.captures(line) else {
            return Err(damaged("Error reading duration!"));
        };

        captures[1]
            .parse::<f64>()
            .map_err(|_| damaged("Error reading duration!"))?
    } else {
        0.0
    };

    // Python's dot is deliberately not escaped here, matching ffmpeg's
    // occasionally non-standard separators.
    let duration_regex = Regex::new(r"[0-9]+:[0-9][0-9]:[0-9][0-9].[0-9][0-9]")
        .expect("the duration regex is valid");

    let Some(found) = duration_regex.find(line) else {
        return Err(damaged("Error reading duration!"));
    };

    let mut hms = Vec::new();

    for piece in line[found.start()..found.end()].split(':') {
        let value = piece
            .parse::<f64>()
            .map_err(|_| damaged("Error reading duration!"))?;

        hms.push(value);
    }

    let duration_s = match hms.as_slice() {
        [seconds] => *seconds,
        [minutes, seconds] => 60.0 * minutes + seconds,
        [hours, minutes, seconds] => 3600.0 * hours + 60.0 * minutes + seconds,
        _ => 0.0,
    };

    if duration_s == 0.0 {
        return Ok((None, None));
    }

    if start_offset > 0.85 * duration_s {
        // Example: Duration: 127:57:31.25, start: 460633.291000.
        return Ok((None, None));
    }

    let start_offset = if start_offset > 1.0 {
        0.0
    } else {
        start_offset
    };

    // Keep the Python variables explicit; they are used to detect videos with
    // an unusual start offset.
    let file_duration_s = duration_s + start_offset;
    let stream_duration_s = duration_s;

    Ok((Some(file_duration_s), Some(stream_duration_s)))
}

/// `parse_ffmpeg_fps_possible_results( video_line )`
///
/// Python returns a set. This returns a sorted, de-duplicated vector.
pub fn parse_ffmpeg_fps_possible_results(video_line: &str) -> (Vec<f64>, bool) {
    fn that_fps_string_is_likely_stupid(fps: &str) -> bool {
        if fps.ends_with('k') {
            return true;
        }

        match fps.parse::<f64>() {
            Ok(fps) => fps <= 1.0 || fps == 100.0 || fps > 144.0,
            Err(_) => true,
        }
    }

    let mut possible_results: Vec<f64> = Vec::new();

    for pattern in [
        r"([0-9]+(?:\.[0-9]+)?)\s+tbr",
        r"([0-9]+(?:\.[0-9]+)?)\s+fps",
    ] {
        let regex = Regex::new(pattern).expect("the fps regex is valid");

        if let Some(captures) = regex.captures(video_line) {
            let text = &captures[1];

            if !that_fps_string_is_likely_stupid(text) {
                if let Ok(value) = text.parse::<f64>() {
                    possible_results.push(value);
                }
            }
        }
    }

    possible_results.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    possible_results.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);
    possible_results.retain(|value| *value != 0.0);

    let confident = if possible_results.is_empty() {
        false
    } else {
        let max_fps = possible_results
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        if possible_results
            .iter()
            .all(|possible_fps| *possible_fps >= max_fps * 0.95)
        {
            true
        } else {
            possible_results.len() <= 1
        }
    };

    (possible_results, confident)
}

/// `parse_ffmpeg_video_line( lines, png_ok = False )`
pub fn parse_ffmpeg_video_line(lines: &[String], png_ok: bool) -> HydrusResult<String> {
    let pure_video_lines = hydrus_ffmpeg_parsing::get_video_stream_lines(lines);

    let bad_video_formats: &[&str] = if png_ok { &["jpg"] } else { &["png", "jpg"] };

    let video_lines: Vec<&String> = pure_video_lines
        .iter()
        .filter(|line| {
            !bad_video_formats
                .iter()
                .any(|format| line.contains(&format!("Video: {format}")))
        })
        .collect();

    if video_lines.is_empty() {
        return Err(damaged("Could not find video information!"));
    }

    let fps_looks_good = |line: &str| {
        let (possible_results, confident) = parse_ffmpeg_fps_possible_results(line);

        possible_results.iter().any(|value| *value != 1.0) && confident
    };

    if let Some(line) = video_lines
        .iter()
        .find(|line| fps_looks_good(line.as_str()))
    {
        return Ok((*line).clone());
    }

    Ok(video_lines[0].clone())
}

/// `parse_ffmpeg_fps( lines, png_ok = False )`
pub fn parse_ffmpeg_fps(lines: &[String], png_ok: bool) -> HydrusResult<(f64, bool)> {
    let line = parse_ffmpeg_video_line(lines, png_ok)?;

    let (possible_results, confident) = parse_ffmpeg_fps_possible_results(&line);

    let fps = possible_results.first().copied().unwrap_or(1.0);

    Ok((fps, confident))
}

/// `parse_ffmpeg_has_video( lines )`
pub fn parse_ffmpeg_has_video(lines: &[String]) -> bool {
    parse_ffmpeg_video_line(lines, false).is_ok()
}

/// `parse_ffmpeg_metadata_container( lines )`
pub fn parse_ffmpeg_metadata_container(lines: &[String]) -> String {
    let metadata_regex = Regex::new(r"^\s*Metadata:\s*").expect("the metadata regex is valid");

    let Some(metadata_line_index) = lines.iter().position(|line| metadata_regex.is_match(line))
    else {
        return String::new();
    };

    let major_brand_regex =
        Regex::new(r"^\s*major_brand\s*:.+").expect("the major_brand regex is valid");

    for line in &lines[metadata_line_index..] {
        if major_brand_regex.is_match(line) {
            return line
                .split_once(':')
                .map(|(_, value)| value.trim().to_string())
                .unwrap_or_default();
        }
    }

    String::new()
}

/// `parse_ffmpeg_mime_text( lines )`
pub fn parse_ffmpeg_mime_text(lines: &[String]) -> HydrusResult<String> {
    let input_line = lines
        .iter()
        .find(|line| line.starts_with("Input #0"))
        .ok_or_else(|| damaged("Error reading file type!"))?;

    if input_line.len() < 10 {
        return Err(damaged("Error reading file type!"));
    }

    let text = &input_line[10..];

    Ok(text.split(", from").next().unwrap_or_default().to_string())
}

/// `parse_ffmpeg_num_frames_manually( lines )`
pub fn parse_ffmpeg_num_frames_manually(lines: &[String]) -> HydrusResult<u32> {
    let frame_lines: Vec<&String> = lines
        .iter()
        .filter(|line| line.starts_with("frame="))
        .collect();

    let Some(final_line) = frame_lines.last() else {
        return Err(damaged(
            "Video appears to be broken and non-renderable--perhaps a damaged single-frame video?",
        ));
    };

    let line = final_line.replacen("frame=", "", 1);
    let line = line.trim_start();

    let frames_string = line.split(' ').next().unwrap_or_default();

    frames_string.parse::<u32>().map_err(|_| {
        damaged(format!(
            "Video was unable to render correctly--could not parse ffmpeg output line: \"{}\"",
            final_line
        ))
    })
}

/// `parse_ffmpeg_video_format( lines )`
pub fn parse_ffmpeg_video_format(lines: &[String]) -> (bool, String, String) {
    let mut video_format = "unknown".to_string();
    let mut stream_mapping = "0:0".to_string();

    let Ok(line) = parse_ffmpeg_video_line(lines, false) else {
        return (false, "unknown".to_string(), stream_mapping);
    };

    let format_regex =
        Regex::new(r"Video:\s*(.+?)(?:,|$)").expect("the video format regex is valid");

    if let Some(captures) = format_regex.captures(&line) {
        video_format = captures[1].trim().to_string();

        if video_format.starts_with("none") {
            return (false, "none".to_string(), stream_mapping);
        }
    }

    if let Ok(mapping) = hydrus_ffmpeg_parsing::parse_stream_mapping_from_stream_line(&line) {
        stream_mapping = mapping;
    }

    (true, video_format, stream_mapping)
}

/// `parse_ffmpeg_video_resolution( lines, png_ok = False )`
pub fn parse_ffmpeg_video_resolution(lines: &[String], png_ok: bool) -> HydrusResult<(u32, u32)> {
    let line = parse_ffmpeg_video_line(lines, png_ok)?;

    let resolution_regex =
        Regex::new(r" ([0-9]+)x([0-9]+)[, ]").expect("the resolution regex is valid");

    let Some(captures) = resolution_regex.captures(&line) else {
        return Err(damaged("Error parsing resolution!"));
    };

    let mut width = captures[1]
        .parse::<u32>()
        .map_err(|_| damaged("Error parsing resolution!"))?;
    let height = captures[2]
        .parse::<u32>()
        .map_err(|_| damaged("Error parsing resolution!"))?;

    let sar_regex =
        Regex::new(r"[\[\s]SAR\s+([0-9]+):([0-9]+)[,\s]").expect("the SAR regex is valid");

    if let Some(captures) = sar_regex.captures(&line) {
        let sar_width = captures[1]
            .parse::<u32>()
            .map_err(|_| damaged("Error parsing resolution!"))?;
        let sar_height = captures[2]
            .parse::<u32>()
            .map_err(|_| damaged("Error parsing resolution!"))?;

        if sar_height != 0 {
            width = width.saturating_mul(sar_width) / sar_height;
        }
    }

    let rotation_regex =
        Regex::new(r"(displaymatrix:|Display Matrix:) rotation of -?90\.00 degrees")
            .expect("the rotation regex is valid");

    if lines.iter().any(|line| rotation_regex.is_match(line)) {
        return Ok((height, width));
    }

    Ok((width, height))
}

/// `video_has_audio( path, info_lines )`
///
/// Some videos carry silent audio streams, so this renders the audio as PCM and
/// looks for any non-silent sample. The Python tolerance of 5..=250 is kept.
pub fn video_has_audio(path: impl AsRef<Path>, info_lines: &[String]) -> HydrusResult<bool> {
    let (audio_found, _, _, _) = hydrus_audio_handling::parse_ffmpeg_audio(info_lines);

    if !audio_found {
        return Ok(false);
    }

    let path = path.as_ref();

    let ffmpeg_path = hydrus_ffmpeg::get_current_ffmpeg_path();

    let cmd = SubprocessCommand::args([
        ffmpeg_path,
        "-i".to_string(),
        path.to_string_lossy().into_owned(),
        "-loglevel".to_string(),
        "quiet".to_string(),
        "-f".to_string(),
        "s16le".to_string(),
        "-".to_string(),
    ]);

    crate::hydrus_data::check_program_is_not_shutting_down()?;

    let streamer = SubprocessContext::new(
        cmd,
        hydrus_ffmpeg::FFMPEG_SUBPROCESS_TIMEOUT,
        65536,
        true,
        false,
        false,
    );

    let streamer = match streamer {
        Ok(streamer) => streamer,
        Err(error) => {
            if error.is_a(ErrorKind::Executable) {
                return Err(
                    hydrus_ffmpeg::handle_ffmpeg_file_not_found_and_generate_exception(
                        &error, path,
                    ),
                );
            }

            return Err(error);
        }
    };

    for chunk_of_pcm_data in streamer.iterate_chunks()? {
        if chunk_of_pcm_data
            .iter()
            .any(|byte| (5..=250).contains(byte))
        {
            return Ok(true);
        }
    }

    Ok(false)
}

/// One raw frame from [`VideoRendererFFMPEG`].
///
/// Python returns a numpy array shaped `(height, width, depth)`. Keeping the
/// bytes and dimensions explicit avoids a dependency on an array crate, and
/// [`VideoFrame::to_dynamic_image`] supplies the image representation used by
/// the rest of the crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub depth: u8,
}

impl VideoFrame {
    pub fn new(data: Vec<u8>, width: u32, height: u32, depth: u8) -> HydrusResult<Self> {
        if depth != 3 && depth != 4 {
            return Err(HydrusException::new(
                ErrorKind::Unknown,
                "Video frame depth must be three or four.",
            ));
        }

        let expected = width as usize * height as usize * depth as usize;

        if data.len() != expected {
            return Err(HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                "Video frame data had the wrong length.",
            ));
        }

        Ok(Self {
            data,
            width,
            height,
            depth,
        })
    }

    pub fn to_dynamic_image(&self) -> HydrusResult<DynamicImage> {
        if self.depth == 4 {
            RgbaImage::from_raw(self.width, self.height, self.data.clone())
                .map(DynamicImage::ImageRgba8)
                .ok_or_else(|| {
                    HydrusException::new(
                        ErrorKind::DamagedOrUnusualFile,
                        "Could not build an RGBA video frame.",
                    )
                })
        } else {
            RgbImage::from_raw(self.width, self.height, self.data.clone())
                .map(DynamicImage::ImageRgb8)
                .ok_or_else(|| {
                    HydrusException::new(
                        ErrorKind::DamagedOrUnusualFile,
                        "Could not build an RGB video frame.",
                    )
                })
        }
    }
}

/// `class VideoRendererFFMPEG`
///
/// The renderer keeps an ffmpeg process producing raw frames. Like the Python
/// version, it can start at an arbitrary frame, skip forward, and rewind by
/// restarting ffmpeg.
pub struct VideoRendererFFMPEG {
    path: std::path::PathBuf,
    mime: Mime,
    duration_s: f64,
    num_frames: u32,
    target_resolution: (u32, u32),
    clip_rect: Option<(u32, u32, u32, u32)>,
    have_selected_an_explicit_video_stream_mapping: bool,
    video_stream_mapping: Option<String>,
    pix_fmt: &'static str,
    depth: u8,
    bufsize: usize,
    process_reader: Option<SubprocessContext>,
    lastread: Option<VideoFrame>,
    fps: f64,
    pos: u32,
}

impl VideoRendererFFMPEG {
    /// `VideoRendererFFMPEG.__init__( path, mime, duration_ms, num_frames, target_resolution, clip_rect = None, start_pos = None )`
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        path: impl AsRef<Path>,
        mime: Mime,
        duration_ms: Option<i64>,
        num_frames: u32,
        target_resolution: (u32, u32),
        clip_rect: Option<(u32, u32, u32, u32)>,
        start_pos: Option<u32>,
    ) -> HydrusResult<Self> {
        let duration_ms = match duration_ms {
            Some(duration_ms) if duration_ms > 0 => duration_ms,
            _ => 100,
        };

        let duration_s = hydrus_time::secondise_ms_float(Some(duration_ms as f64)).unwrap_or(0.1);

        let pix_fmt = if MIMES_THAT_WE_CAN_CHECK_FOR_TRANSPARENCY.contains(&mime) {
            "rgba"
        } else {
            "rgb24"
        };

        let depth = if pix_fmt == "rgba" { 4 } else { 3 };

        let (width, height) = target_resolution;

        let bufsize = depth as usize * width as usize * height as usize;

        let fps = f64::from(num_frames) / duration_s;

        let fps = if fps == 0.0 { 24.0 } else { fps };

        let mut renderer = Self {
            path: path.as_ref().to_path_buf(),
            mime,
            duration_s,
            num_frames,
            target_resolution,
            clip_rect,
            have_selected_an_explicit_video_stream_mapping: false,
            video_stream_mapping: None,
            pix_fmt,
            depth,
            bufsize,
            process_reader: None,
            lastread: None,
            fps,
            pos: 0,
        };

        renderer.initialize(start_pos.unwrap_or(0))?;

        Ok(renderer)
    }

    /// `close()`
    pub fn close(&mut self) {
        if let Some(process_reader) = self.process_reader.take() {
            process_reader.close_process();
        }
    }

    /// `initialize( start_index = 0 )`
    pub fn initialize(&mut self, start_index: u32) -> HydrusResult<()> {
        self.close();

        let (do_ss, ss, skip_frames) = if ANIMATIONS.contains(&self.mime) {
            // ffmpeg is bad at scanning animations, and they are short, so
            // skipping frames is more reliable.
            self.pos = 0;
            (false, 0.0, start_index)
        } else {
            let ss = if start_index == 0 {
                0.0
            } else {
                f64::from(start_index) / self.fps
            };

            self.pos = start_index;

            (start_index != 0, ss, 0)
        };

        let (width, height) = self.target_resolution;

        let ffmpeg_path = hydrus_ffmpeg::get_current_ffmpeg_path();

        let mut cmd = vec![ffmpeg_path];

        if do_ss {
            cmd.extend(["-ss".to_string(), format!("{ss:.03}")]);
        }

        cmd.extend(["-i".to_string(), self.path.to_string_lossy().into_owned()]);

        if let Some(video_stream_mapping) = &self.video_stream_mapping {
            cmd.extend(["-map".to_string(), video_stream_mapping.clone()]);
        }

        let mut video_filters = vec![format!("scale={width}:{height}")];

        if let Some((clip_x, clip_y, clip_width, clip_height)) = self.clip_rect {
            video_filters.push(format!("crop={clip_width}:{clip_height}:{clip_x}:{clip_y}"));
        }

        cmd.extend(["-vf".to_string(), video_filters.join(",")]);

        cmd.extend([
            "-loglevel".to_string(),
            "quiet".to_string(),
            "-f".to_string(),
            "rawvideo".to_string(),
            "-pix_fmt".to_string(),
            self.pix_fmt.to_string(),
            "-fps_mode".to_string(),
            "passthrough".to_string(),
            "-vcodec".to_string(),
            "rawvideo".to_string(),
            "-".to_string(),
        ]);

        hydrus_data::check_program_is_not_shutting_down()?;

        let process_reader = match SubprocessContext::new(
            SubprocessCommand::args(cmd),
            hydrus_ffmpeg::FFMPEG_SUBPROCESS_TIMEOUT,
            self.bufsize,
            true,
            false,
            false,
        ) {
            Ok(process_reader) => process_reader,
            Err(error) if error.is_a(ErrorKind::Executable) => {
                return Err(
                    hydrus_ffmpeg::handle_ffmpeg_file_not_found_and_generate_exception(
                        &error, &self.path,
                    ),
                );
            }
            Err(error) => return Err(error),
        };

        self.process_reader = Some(process_reader);

        if skip_frames > 0 {
            self.skip_frames(skip_frames)?;
        }

        Ok(())
    }

    /// `skip_frames( n )`
    ///
    /// A timeout is treated as the end of the stream, matching Python. Other
    /// subprocess errors are returned.
    pub fn skip_frames(&mut self, mut n: u32) -> HydrusResult<()> {
        while n > 0 {
            if let Some(process_reader) = &self.process_reader {
                match process_reader.read_chunk() {
                    Ok(_) => {}
                    Err(error) if error.is_a(ErrorKind::SubprocessTimedOut) => {
                        // A timeout means we have run off the end of the stream.
                        self.process_reader = None;
                    }
                    Err(error) => {
                        self.process_reader = None;
                        return Err(error);
                    }
                }
            }

            self.pos = self.pos.saturating_add(1);
            n -= 1;
        }

        Ok(())
    }

    /// `read_frame()`
    pub fn read_frame(&mut self) -> HydrusResult<VideoFrame> {
        if self.pos == self.num_frames {
            self.initialize(0)?;
        }

        if self.process_reader.is_none() {
            return self
                .lastread
                .clone()
                .ok_or_else(|| damaged("Unable to render that video!"));
        }

        let (width, height) = self.target_resolution;

        let frame_of_data = match self
            .process_reader
            .as_ref()
            .expect("the process reader was checked above")
            .read_chunk()
        {
            Ok(frame) => frame,
            Err(error) => {
                self.process_reader = None;

                if error.is_a(ErrorKind::SubprocessTimedOut) {
                    if let Some(lastread) = &self.lastread {
                        return Ok(lastread.clone());
                    }
                }

                return Err(error);
            }
        };

        let expected_length = width as usize * height as usize * self.depth as usize;

        if frame_of_data.len() != expected_length {
            if self.pos == 1 && !self.have_selected_an_explicit_video_stream_mapping {
                self.have_selected_an_explicit_video_stream_mapping = true;

                let ffmpeg_lines = hydrus_ffmpeg::get_ffmpeg_info_lines(&self.path, None)?;

                let (_, _, video_stream_mapping) = parse_ffmpeg_video_format(&ffmpeg_lines);

                self.video_stream_mapping = Some(video_stream_mapping);

                self.initialize(0)?;

                return self.read_frame();
            }

            if self.lastread.is_none() {
                if self.pos != 0 {
                    // A broken key frame index may have made the requested
                    // mid-video seek impossible. Try the start before giving up.
                    self.set_position(0)?;

                    return self.read_frame();
                }

                let message = if running_from_source() {
                    "Unable to render that video! Please ensure your FFMPEG is updated (hit _help->about_ to check version), and if it the file still fails, please send it to hydrus dev so he can look at it!"
                } else {
                    "Unable to render that video! It is probably just weird/broken, but hydev would be interested in seeing it!"
                };

                return Err(damaged(message));
            }

            let result = self.lastread.clone().expect("lastread was just checked");
            self.close();
            self.pos = self.pos.saturating_add(1);

            return Ok(result);
        }

        let frame = VideoFrame::new(frame_of_data, width, height, self.depth)?;

        self.lastread = Some(frame.clone());
        self.pos = self.pos.saturating_add(1);

        Ok(frame)
    }

    /// `set_position( pos )`
    pub fn set_position(&mut self, pos: u32) -> HydrusResult<()> {
        let rewind = pos < self.pos;
        let jump_a_long_way_ahead = pos > self.pos.saturating_add(60);

        if rewind || jump_a_long_way_ahead {
            self.initialize(pos)?;
        } else {
            self.skip_frames(pos.saturating_sub(self.pos))?;
        }

        Ok(())
    }

    /// `stop()`
    pub fn stop(&mut self) {
        self.close();
    }

    pub fn fps(&self) -> f64 {
        self.fps
    }

    pub fn duration_s(&self) -> f64 {
        self.duration_s
    }
}

impl Drop for VideoRendererFFMPEG {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_lines() -> Vec<String> {
        r#"Input #0, matroska,webm, from 'm.mkv':
  Metadata:
    major_brand     : webm
  Duration: 00:00:02.46, start: 0.033000, bitrate: 1069 kb/s
  Stream #0:0: Video: vp9 (Profile 0), yuv420p(tv, progressive), 1280x720, SAR 1:1 DAR 16:9, 30 fps, 30 tbr, 1k tbn (default)
  Stream #0:1: Audio: opus, 48000 Hz, stereo, fltp (default)
"#
        .lines()
        .map(|line| line.trim_start().to_string())
        .collect()
    }

    #[test]
    fn duration_and_start_are_parsed() {
        let (file_duration, stream_duration) = parse_ffmpeg_duration(&sample_lines()).unwrap();

        assert!((file_duration.unwrap() - 2.493).abs() < 0.001);
        assert!((stream_duration.unwrap() - 2.46).abs() < 0.001);
    }

    #[test]
    fn video_format_and_audio_are_parsed() {
        let lines = sample_lines();

        let (has_video, format, mapping) = parse_ffmpeg_video_format(&lines);

        assert!(has_video);
        assert_eq!(format, "vp9 (Profile 0)");
        assert_eq!(mapping, "0:0");

        let (has_audio, audio_format, _, _) = hydrus_audio_handling::parse_ffmpeg_audio(&lines);

        assert!(has_audio);
        assert_eq!(audio_format, "opus");
    }

    #[test]
    fn resolution_applies_sample_aspect_ratio() {
        assert_eq!(
            parse_ffmpeg_video_resolution(&sample_lines(), false).unwrap(),
            (1280, 720)
        );

        let mut lines = sample_lines();
        lines[4] =
            "Stream #0:0: Video: vp9, yuv420p, 1120x1080, SAR 10:11 DAR 280:297, 30 fps, 30 tbr"
                .to_string();

        assert_eq!(
            parse_ffmpeg_video_resolution(&lines, false).unwrap(),
            (1018, 1080)
        );
    }

    #[test]
    fn raw_video_frames_convert_to_images() {
        let frame = VideoFrame::new(vec![1, 2, 3, 4, 5, 6], 2, 1, 3).unwrap();
        let image = frame.to_dynamic_image().unwrap();

        assert_eq!(image.width(), 2);
        assert_eq!(image.height(), 1);
        assert_eq!(image.to_rgb8().get_pixel(1, 0), &image::Rgb([4, 5, 6]));

        let rgba = VideoFrame::new(vec![1, 2, 3, 4], 1, 1, 4).unwrap();

        assert_eq!(
            rgba.to_dynamic_image().unwrap().color(),
            image::ColorType::Rgba8
        );
        assert!(VideoFrame::new(vec![1, 2, 3], 2, 2, 3).is_err());
        assert!(VideoFrame::new(vec![1, 2, 3], 1, 1, 5).is_err());
    }

    #[test]
    fn rotation_swaps_resolution() {
        let mut lines = sample_lines();
        lines.push("      displaymatrix: rotation of -90.00 degrees".to_string());

        assert_eq!(
            parse_ffmpeg_video_resolution(&lines, false).unwrap(),
            (720, 1280)
        );
    }

    #[test]
    fn fps_prefers_confident_plausible_values() {
        let (values, confident) = parse_ffmpeg_fps_possible_results(
            "Stream #0:0: Video: h264, yuv420p, 1280x720, 29.97 fps, 30 tbr, 90k tbn",
        );

        assert!(confident);
        assert_eq!(values, vec![29.97, 30.0]);
    }

    #[test]
    fn frame_counter_takes_the_last_progress_line() {
        let lines = vec![
            "frame=  10 fps=0.0".to_string(),
            "frame= 400 fps=100.0".to_string(),
        ];

        assert_eq!(parse_ffmpeg_num_frames_manually(&lines).unwrap(), 400);
    }

    #[test]
    fn metadata_container_is_read() {
        assert_eq!(parse_ffmpeg_metadata_container(&sample_lines()), "webm");
    }

    #[test]
    fn mime_text_is_read() {
        assert_eq!(
            parse_ffmpeg_mime_text(&sample_lines()).unwrap(),
            "matroska,webm"
        );
    }
}
