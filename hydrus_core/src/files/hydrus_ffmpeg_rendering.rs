//! Rust port of `hydrus/core/files/HydrusFFMPEGRendering.py`.
//!
//! All rendering is delegated to the `ffmpeg` executable. The functions here
//! only build the right command and translate timeout/missing-program errors.

use std::path::Path;

use crate::hydrus_exceptions::HydrusResult;
use crate::files::hydrus_ffmpeg;
use crate::files::hydrus_ffmpeg_parsing;

/// Build the command that extracts an attached still image.
pub fn build_attached_still_command(
    file_path: &Path,
    output_path: &Path,
    image_stream_mapping: &str,
) -> Vec<String> {
    vec![
        hydrus_ffmpeg::get_current_ffmpeg_path(),
        "-xerror".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        file_path.to_string_lossy().into_owned(),
        "-map".to_string(),
        image_stream_mapping.to_string(),
        "-frames:v".to_string(),
        "1".to_string(),
        "-update".to_string(),
        "1".to_string(),
        "-f".to_string(),
        "image2".to_string(),
        "-c:v".to_string(),
        "png".to_string(),
        output_path.to_string_lossy().into_owned(),
    ]
}

/// `render_any_attached_still_image_to_path( file_path, output_path )`
pub fn render_any_attached_still_image_to_path(
    file_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> HydrusResult<bool> {
    let file_path = file_path.as_ref();
    let output_path = output_path.as_ref();

    let lines = hydrus_ffmpeg::get_ffmpeg_info_lines(file_path, None)?;
    let image_lines = hydrus_ffmpeg_parsing::get_image_stream_lines(&lines);

    let Some(image_line) = image_lines.first() else {
        return Ok(false);
    };

    let Ok(image_stream_mapping) =
        hydrus_ffmpeg_parsing::parse_stream_mapping_from_stream_line(image_line)
    else {
        return Ok(false);
    };

    let command = build_attached_still_command(file_path, output_path, &image_stream_mapping);

    hydrus_ffmpeg::run_ffmpeg_command(
        &command,
        false,
        1024 * 512,
        file_path,
        "ffmpeg could not render it quick enough!",
    )?;

    Ok(true)
}

/// `render_image_to_image_path( path, temp_image_path )`
pub fn render_image_to_image_path(
    path: impl AsRef<Path>,
    temp_image_path: impl AsRef<Path>,
) -> HydrusResult<()> {
    let path = path.as_ref();
    let temp_image_path = temp_image_path.as_ref();

    let mut command = vec![
        hydrus_ffmpeg::get_current_ffmpeg_path(),
        "-xerror".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        path.to_string_lossy().into_owned(),
    ];

    if temp_image_path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("jpg"))
    {
        command.extend(["-q:v".to_string(), "1".to_string()]);
    }

    command.push(temp_image_path.to_string_lossy().into_owned());

    hydrus_ffmpeg::run_ffmpeg_command(
        &command,
        true,
        65_536,
        path,
        "ffmpeg could not render it quick enough!",
    )?;

    Ok(())
}

/// `render_image_to_raw_rgba_bytes( path )`
pub fn render_image_to_raw_rgba_bytes(path: impl AsRef<Path>) -> HydrusResult<Vec<u8>> {
    let path = path.as_ref();

    let command = vec![
        hydrus_ffmpeg::get_current_ffmpeg_path(),
        "-xerror".to_string(),
        "-i".to_string(),
        path.to_string_lossy().into_owned(),
        "-f".to_string(),
        "rawvideo".to_string(),
        "-pix_fmt".to_string(),
        "rgba".to_string(),
        "-".to_string(),
    ];

    let result = hydrus_ffmpeg::run_ffmpeg_command(
        &command,
        false,
        1024 * 512,
        path,
        "ffmpeg could not render it quick enough!",
    )?;

    Ok(result.stdout.unwrap_or_default())
}

/// `render_image_to_png_bytes( path )`
pub fn render_image_to_png_bytes(path: impl AsRef<Path>) -> HydrusResult<Vec<u8>> {
    let path = path.as_ref();

    let command = vec![
        hydrus_ffmpeg::get_current_ffmpeg_path(),
        "-xerror".to_string(),
        "-i".to_string(),
        path.to_string_lossy().into_owned(),
        "-f".to_string(),
        "image2pipe".to_string(),
        "-vcodec".to_string(),
        "png".to_string(),
        "-".to_string(),
    ];

    let result = hydrus_ffmpeg::run_ffmpeg_command(
        &command,
        false,
        1024 * 512,
        path,
        "ffmpeg could not render it quick enough!",
    )?;

    Ok(result.stdout.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attached_still_command_selects_the_stream_and_writes_png() {
        let command = build_attached_still_command(
            Path::new("in.mp3"),
            Path::new("out.png"),
            "0:1",
        );

        assert_eq!(command[4], "in.mp3");
        assert!(command.windows(2).any(|pair| pair == ["-map", "0:1"]));
        assert!(command.windows(2).any(|pair| pair == ["-frames:v", "1"]));
        assert!(command.windows(2).any(|pair| pair == ["-c:v", "png"]));
        assert_eq!(command.last().map(String::as_str), Some("out.png"));
    }
}
