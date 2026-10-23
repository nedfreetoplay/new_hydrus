//! Rust port of `hydrus/core/files/HydrusAudioHandling.py`.
//!
//! The audio parser only needs to answer "is there audio, what does it call
//! itself, and which FFMPEG stream should be used?".

use std::sync::OnceLock;

use regex::Regex;

use crate::files::hydrus_ffmpeg_parsing;

fn audio_format_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();

    REGEX.get_or_init(|| Regex::new(r"Audio:\s*(.+?),").unwrap())
}

/// `parse_ffmpeg_audio( lines )`
///
/// Returns `( audio_found, audio_format, audio_stream_mapping,
/// image_stream_mapping )`. The image mapping is present when an attached
/// cover or a single-frame image stream was found.
pub fn parse_ffmpeg_audio(lines: &[String]) -> (bool, String, String, Option<String>) {
    let audio_lines = hydrus_ffmpeg_parsing::get_audio_stream_lines(lines);

    let audio_found = !audio_lines.is_empty();
    let mut audio_format = "unknown".to_string();
    let mut audio_stream_mapping = "0:0".to_string();
    let mut image_stream_mapping = None;

    let Some(audio_line) = audio_lines.first() else {
        return (
            audio_found,
            audio_format,
            audio_stream_mapping,
            image_stream_mapping,
        );
    };

    if let Some(captures) = audio_format_regex().captures(audio_line) {
        audio_format = captures[1].to_string();
    }

    if let Ok(mapping) =
        hydrus_ffmpeg_parsing::parse_stream_mapping_from_stream_line(audio_line)
    {
        audio_stream_mapping = mapping;
    }

    let image_lines = hydrus_ffmpeg_parsing::get_image_stream_lines(lines);

    if let Some(image_line) = image_lines.first() {
        if let Ok(mapping) =
            hydrus_ffmpeg_parsing::parse_stream_mapping_from_stream_line(image_line)
        {
            image_stream_mapping = Some(mapping);
        }
    }

    (
        audio_found,
        audio_format,
        audio_stream_mapping,
        image_stream_mapping,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audio_lines() -> Vec<String> {
        vec![
            "Input #0, mp3, from 'example.mp3':".to_string(),
            "Stream #0:0: Audio: mp3 (mp3float), 44100 Hz, stereo, fltp, 192 kb/s".to_string(),
        ]
    }

    #[test]
    fn audio_is_found_with_its_format_and_mapping() {
        let (found, format, mapping, image) = parse_ffmpeg_audio(&audio_lines());

        assert!(found);
        assert_eq!(format, "mp3 (mp3float)");
        assert_eq!(mapping, "0:0");
        assert_eq!(image, None);
    }

    #[test]
    fn attached_image_is_reported_separately() {
        let mut lines = audio_lines();

        lines.push(
            "Stream #0:1: Video: png, rgb24(pc), 300x300, 90k tbr (attached pic)".to_string(),
        );

        let (found, _format, audio_mapping, image_mapping) = parse_ffmpeg_audio(&lines);

        assert!(found);
        assert_eq!(audio_mapping, "0:0");
        assert_eq!(image_mapping.as_deref(), Some("0:1"));
    }

    #[test]
    fn no_audio_keeps_the_defaults() {
        let lines = vec![
            "Stream #0:0: Video: hevc (Main), yuv420p, 256x144, 25 fps".to_string(),
        ];

        let (found, format, mapping, image) = parse_ffmpeg_audio(&lines);

        assert!(!found);
        assert_eq!(format, "unknown");
        assert_eq!(mapping, "0:0");
        assert_eq!(image, None);
    }

    #[test]
    fn missing_mapping_falls_back_to_zero_zero() {
        let lines = vec!["Stream Audio: opus, 48000 Hz".to_string()];

        let (found, format, mapping, image) = parse_ffmpeg_audio(&lines);

        assert!(found);
        assert_eq!(format, "opus");
        assert_eq!(mapping, "0:0");
        assert_eq!(image, None);
    }
}
