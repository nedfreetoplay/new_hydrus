//! Rust port of `hydrus/core/files/HydrusFFMPEGParsing.py`.
//!
//! FFMPEG writes stream information to stderr. These helpers are deliberately
//! dumb line filters, matching the Python code rather than trying to parse the
//! whole media description.

use std::sync::OnceLock;

use regex::Regex;

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// `get_audio_stream_lines( lines )`
pub fn get_audio_stream_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| line.starts_with("Stream ") && line.contains("Audio: "))
        .cloned()
        .collect()
}

/// `get_video_stream_lines( lines )`
///
/// This includes single-frame PNG streams and attached cover art.
pub fn get_video_stream_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| line.starts_with("Stream ") && line.contains("Video: "))
        .cloned()
        .collect()
}

/// `get_image_stream_lines( lines )`
pub fn get_image_stream_lines(lines: &[String]) -> Vec<String> {
    get_video_stream_lines(lines)
        .into_iter()
        .filter(|line| {
            line.contains("Video: png")
                || line.contains("Video: jpg")
                || line.contains("attached pic")
                || line.contains("attached_pic")
        })
        .collect()
}

fn stream_mapping_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();

    REGEX.get_or_init(|| Regex::new(r"^Stream #(\d+:\d+)").unwrap())
}

/// `parse_stream_mapping_from_stream_line( line )`
pub fn parse_stream_mapping_from_stream_line(line: &str) -> HydrusResult<String> {
    let Some(captures) = stream_mapping_regex().captures(line) else {
        return Err(HydrusException::new(
            ErrorKind::DataMissing,
            "No stream mapping found!",
        ));
    };

    Ok(captures[1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines() -> Vec<String> {
        vec![
            "Input #0, mp3, from 'example.mp3':".to_string(),
            "title           : Audio: not a stream line".to_string(),
            "Stream #0:0: Audio: mp3 (mp3float), 44100 Hz, stereo, fltp, 192 kb/s".to_string(),
            "Stream #0:1: Video: png, rgb24(pc), 300x300, 90k tbr (attached pic)".to_string(),
            "Stream #0:2: Video: hevc (Main), yuv420p, 256x144, 25 fps".to_string(),
        ]
    }

    #[test]
    fn audio_lines_only_include_stream_entries() {
        let audio = get_audio_stream_lines(&lines());

        assert_eq!(audio.len(), 1);
        assert!(audio[0].contains("Audio: mp3"));
    }

    #[test]
    fn video_lines_include_images_and_normal_video() {
        let video = get_video_stream_lines(&lines());

        assert_eq!(video.len(), 2);
        assert!(video.iter().all(|line| line.contains("Video: ")));
    }

    #[test]
    fn image_lines_include_png_jpg_and_attached_pictures() {
        let images = get_image_stream_lines(&lines());

        assert_eq!(images.len(), 1);
        assert!(images[0].contains("attached pic"));
    }

    #[test]
    fn stream_mapping_is_parsed_from_the_prefix() {
        assert_eq!(
            parse_stream_mapping_from_stream_line(
                "Stream #0:1[0x1](eng): Video: hevc (Main), yuv420p, 256x144"
            )
            .unwrap(),
            "0:1"
        );

        assert_eq!(
            parse_stream_mapping_from_stream_line("Stream #12:3: Audio: opus").unwrap(),
            "12:3"
        );
    }

    #[test]
    fn a_missing_mapping_is_data_missing() {
        let error = parse_stream_mapping_from_stream_line("  Stream 0:1: Video: png").unwrap_err();

        assert!(error.is_a(ErrorKind::DataMissing));
        assert_eq!(error.to_string(), "No stream mapping found!");
    }
}
