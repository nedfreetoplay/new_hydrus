//! Rust port of `hydrus/core/files/HydrusAnimationHandling.py`.
//!
//! Python asks PIL how many frames an animation has and how long each one
//! lasts. There is no PIL here, so this module parses the small amount of
//! container metadata directly:
//!
//! * APNG: `acTL` and `fcTL` chunks;
//! * GIF: Graphic Control Extension delays and the NETSCAPE loop extension;
//! * animated WebP: `ANIM` and `ANMF` chunks.
//!
//! The 83 ms / 12 fps fallback is deliberately preserved.

use std::path::Path;

use image::GenericImageView;

use crate::hydrus_constants::Mime;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_image_handling;

const FALLBACK_FRAME_DURATION_MS: u32 = 83;
const APNG_FALLBACK_FRAME_DURATION_S: f64 = 0.1;
const APNG_MIN_FRAME_DURATION_S: f64 = 0.001;

fn damaged(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DamagedOrUnusualFile, message)
}

fn u16_le(bytes: &[u8]) -> Option<u16> {
    Some(u16::from_le_bytes([*bytes.first()?, *bytes.get(1)?]))
}

fn u32_be(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes([
        *bytes.first()?,
        *bytes.get(1)?,
        *bytes.get(2)?,
        *bytes.get(3)?,
    ]))
}

fn u24_le(bytes: &[u8]) -> Option<u32> {
    Some(
        u32::from(*bytes.first()?)
            | (u32::from(*bytes.get(1)?) << 8)
            | (u32::from(*bytes.get(2)?) << 16),
    )
}

/// `get_animation_properties( path, mime )`
pub fn get_animation_properties(
    path: impl AsRef<Path>,
    mime: Mime,
) -> HydrusResult<((u32, u32), i64, u32)> {
    let path = path.as_ref();

    let image = hydrus_image_handling::decode_image_path(path)?;
    let (width, height) = image.dimensions();

    let width = width.max(1);
    let height = height.max(1);

    let (duration_ms, num_frames) = if mime == Mime::Apng {
        let (duration_ms, num_frames) = get_apng_duration_ms_and_num_frames(path)?;

        (duration_ms, num_frames)
    } else if mime == Mime::AnimationWebp {
        let (frame_durations_ms, _) = get_webp_frame_durations_ms(path)?;

        (
            i64::from(frame_durations_ms.iter().sum::<u32>()),
            frame_durations_ms.len() as u32,
        )
    } else {
        let (frame_durations_ms, _) = get_frame_durations_ms_pil_animation(path, None)?;

        (
            i64::from(frame_durations_ms.iter().sum::<u32>()),
            frame_durations_ms.len() as u32,
        )
    };

    Ok(((width, height), duration_ms, num_frames))
}

/// `get_apng_chunks( file_header_bytes )`
///
/// Returns owned chunk data because APNG files can be large and callers often
/// need the data after the original buffer changes.
pub fn get_apng_chunks(file_header_bytes: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    let mut chunks = Vec::new();
    let mut offset = 8;

    while offset + 8 <= file_header_bytes.len() {
        let Some(data_length) = u32_be(&file_header_bytes[offset..offset + 4]) else {
            break;
        };

        let mut name = [0_u8; 4];
        name.copy_from_slice(&file_header_bytes[offset + 4..offset + 8]);

        let data_start = offset + 8;
        let Some(data_end) = data_start.checked_add(data_length as usize) else {
            break;
        };

        if data_end > file_header_bytes.len() {
            break;
        }

        chunks.push((name, file_header_bytes[data_start..data_end].to_vec()));

        // Four bytes of CRC.
        offset = match data_end.checked_add(4) {
            Some(next) => next,
            None => break,
        };
    }

    chunks
}

/// `get_apng_actl_chunk_data( file_header_bytes )`
pub fn get_apng_actl_chunk_data(file_header_bytes: &[u8]) -> Option<Vec<u8>> {
    get_apng_chunks(file_header_bytes)
        .into_iter()
        .find(|(name, _)| name == b"acTL")
        .map(|(_, data)| data)
}

/// `get_apng_duration_ms( apng_bytes ) -> float`
pub fn get_apng_duration_ms(apng_bytes: &[u8]) -> i64 {
    let chunks = get_apng_chunks(apng_bytes);

    let mut total_duration_s = 0.0;

    for (name, data) in chunks {
        if name != *b"fcTL" || data.len() < 24 {
            continue;
        }

        let delay_numerator = u16::from_be_bytes([data[20], data[21]]) as f64;
        let delay_denominator = u16::from_be_bytes([data[22], data[23]]) as f64;

        let duration_s = if delay_denominator == 0.0 {
            APNG_FALLBACK_FRAME_DURATION_S
        } else {
            (delay_numerator / delay_denominator).max(APNG_MIN_FRAME_DURATION_S)
        };

        total_duration_s += duration_s;
    }

    let duration_ms_float = total_duration_s * 1000.0;
    let mut duration_ms = duration_ms_float as i64;

    if duration_ms == 0 && duration_ms_float > 0.0 {
        duration_ms = 1;
    }

    duration_ms
}

/// `get_apng_num_frames( apng_actl_bytes )`
pub fn get_apng_num_frames(apng_actl_bytes: &[u8]) -> u32 {
    u32_be(apng_actl_bytes).unwrap_or(0)
}

/// `get_apng_duration_ms_and_num_frames( path )`
pub fn get_apng_duration_ms_and_num_frames(
    path: impl AsRef<Path>,
) -> HydrusResult<(i64, u32)> {
    let bytes = std::fs::read(path.as_ref()).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!(
                "Could not read the APNG at \"{}\": {error}",
                path.as_ref().display()
            ),
        )
    })?;

    let header = &bytes[..bytes.len().min(256)];

    let Some(apng_actl_bytes) = get_apng_actl_chunk_data(header) else {
        return Err(damaged("This APNG had an unusual file header!"));
    };

    let num_frames = get_apng_num_frames(&apng_actl_bytes);

    Ok((get_apng_duration_ms(&bytes), num_frames))
}

/// `get_frame_durations_ms_pil_animation( path, human_file_description = None )`
pub fn get_frame_durations_ms_pil_animation(
    path: impl AsRef<Path>,
    _human_file_description: Option<&str>,
) -> HydrusResult<(Vec<u32>, u32)> {
    let bytes = std::fs::read(path.as_ref()).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!(
                "Could not read the animation at \"{}\": {error}",
                path.as_ref().display()
            ),
        )
    })?;

    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        gif_frame_durations_ms(&bytes)
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        let (durations, times_to_play) = webp_frame_durations_ms(&bytes);

        Ok((durations, times_to_play))
    } else {
        // An APNG can reach this fallback in Python as well.
        let (duration_ms, num_frames) = get_apng_duration_ms_and_num_frames(path.as_ref())?;

        if num_frames == 0 {
            return Ok((Vec::new(), 1));
        }

        let per_frame = (duration_ms.max(0) / i64::from(num_frames)).max(1) as u32;

        Ok((vec![per_frame; num_frames as usize], 1))
    }
}

fn skip_gif_sub_blocks(bytes: &[u8], mut offset: usize) -> usize {
    while offset < bytes.len() {
        let size = bytes[offset] as usize;
        offset += 1;

        if size == 0 {
            break;
        }

        offset = offset.saturating_add(size);

        if offset > bytes.len() {
            break;
        }
    }

    offset
}

fn gif_frame_durations_ms(bytes: &[u8]) -> HydrusResult<(Vec<u32>, u32)> {
    if bytes.len() < 13 {
        return Err(damaged("GIF was too short to read."));
    }

    let packed = bytes[10];
    let mut offset: usize = 13;

    if packed & 0x80 != 0 {
        let table_size = 3 * (1usize << (u32::from(packed & 0x07) + 1));
        offset = offset.saturating_add(table_size);
    }

    let mut durations = Vec::new();
    let mut current_delay: Option<u32> = None;
    let mut times_to_play = 1_u32;

    while offset < bytes.len() {
        match bytes[offset] {
            0x3B => break,
            0x21 => {
                let Some(&label) = bytes.get(offset + 1) else {
                    break;
                };

                offset += 2;

                if label == 0xF9 {
                    // Graphic Control Extension: block size (4), packed,
                    // delay low/high, transparent index, terminator.
                    if offset + 6 > bytes.len() {
                        break;
                    }

                    let block_size = bytes[offset] as usize;

                    if block_size >= 4 && offset + 1 + 4 <= bytes.len() {
                        let delay = u16::from_le_bytes([
                            bytes[offset + 2],
                            bytes[offset + 3],
                        ]);

                        current_delay = Some(if delay == 0 || delay == 1 {
                            // GIF delay 10 (1/100 s = 10 ms) and 0 are both
                            // commonly broken; PIL treats both as "missing".
                            FALLBACK_FRAME_DURATION_MS
                        } else {
                            u32::from(delay) * 10
                        });
                    }

                    offset = skip_gif_sub_blocks(bytes, offset);
                } else if label == 0xFF {
                    let Some(&block_size) = bytes.get(offset) else {
                        break;
                    };

                    let app_start = offset + 1;
                    let app_end = app_start.saturating_add(block_size as usize);

                    if bytes.get(app_start..app_end).is_some_and(|id| id.starts_with(b"NETSCAPE")) {
                        // Application extension data is a sequence of
                        // sub-blocks; the first one starts with 0x01.
                        let mut data_offset = app_end;

                        if let Some(&sub_size) = bytes.get(data_offset) {
                            if sub_size >= 3 && data_offset + 4 <= bytes.len() {
                                let loop_count = u16::from_le_bytes([
                                    bytes[data_offset + 2],
                                    bytes[data_offset + 3],
                                ]);

                                times_to_play = u32::from(loop_count);
                            }

                            data_offset = skip_gif_sub_blocks(bytes, data_offset);
                        }

                        offset = data_offset;
                    } else {
                        offset = skip_gif_sub_blocks(bytes, offset);
                    }
                } else {
                    offset = skip_gif_sub_blocks(bytes, offset);
                }
            }
            0x2C => {
                // Image descriptor: x/y/width/height/packed, then optional
                // local colour table, LZW minimum code size and sub-blocks.
                if offset + 10 > bytes.len() {
                    break;
                }

                let image_packed = bytes[offset + 9];
                offset += 10;

                if image_packed & 0x80 != 0 {
                    let table_size = 3 * (1usize << (u32::from(image_packed & 0x07) + 1));
                    offset = offset.saturating_add(table_size);
                }

                // LZW minimum code size.
                offset = offset.saturating_add(1);

                durations.push(current_delay.unwrap_or(FALLBACK_FRAME_DURATION_MS));
                current_delay = None;

                offset = skip_gif_sub_blocks(bytes, offset);
            }
            _ => break,
        }
    }

    if durations.is_empty() {
        // A valid GIF with no graphical frames still has a first frame as far
        // as PIL is concerned.
        durations.push(FALLBACK_FRAME_DURATION_MS);
    }

    Ok((durations, times_to_play))
}

/// `get_webp_chunks( file_bytes )`
pub fn get_webp_chunks(file_bytes: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    let mut chunks = Vec::new();
    let mut offset = 12;

    while offset + 8 <= file_bytes.len() {
        let mut chunk_type = [0_u8; 4];
        chunk_type.copy_from_slice(&file_bytes[offset..offset + 4]);

        let chunk_size = u32::from_le_bytes([
            file_bytes[offset + 4],
            file_bytes[offset + 5],
            file_bytes[offset + 6],
            file_bytes[offset + 7],
        ]) as usize;

        let data_start = offset + 8;

        let Some(data_end) = data_start.checked_add(chunk_size) else {
            break;
        };

        if data_end > file_bytes.len() {
            break;
        }

        chunks.push((chunk_type, file_bytes[data_start..data_end].to_vec()));

        let mut total_chunk_size = 8 + chunk_size;

        if total_chunk_size % 2 == 1 {
            total_chunk_size += 1;
        }

        offset = match offset.checked_add(total_chunk_size) {
            Some(next) => next,
            None => break,
        };
    }

    chunks
}

fn webp_frame_durations_ms(file_bytes: &[u8]) -> (Vec<u32>, u32) {
    let mut frame_durations_ms = Vec::new();
    let mut times_to_play = 0;

    for (chunk_type, chunk) in get_webp_chunks(file_bytes) {
        if chunk_type == *b"ANIM" {
            if let Some(loop_count) = chunk.get(4..6).and_then(u16_le) {
                times_to_play = u32::from(loop_count);
            }
        } else if chunk_type == *b"ANMF" {
            let duration_ms = if chunk.len() >= 16 {
                match u24_le(&chunk[12..15]) {
                    Some(0) | None => FALLBACK_FRAME_DURATION_MS,
                    Some(duration) => duration,
                }
            } else {
                FALLBACK_FRAME_DURATION_MS
            };

            frame_durations_ms.push(duration_ms);
        }
    }

    (frame_durations_ms, times_to_play)
}

/// `get_webp_frame_durations_ms( path )`
pub fn get_webp_frame_durations_ms(path: impl AsRef<Path>) -> HydrusResult<(Vec<u32>, u32)> {
    let bytes = std::fs::read(path.as_ref()).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!(
                "Could not read the WebP at \"{}\": {error}",
                path.as_ref().display()
            ),
        )
    })?;

    Ok(webp_frame_durations_ms(&bytes))
}

/// `pil_animation_has_duration( path )`
///
/// Python returns PIL's `is_animated` flag, which is simply "more than one
/// frame". This port reads the same property from the container.
pub fn pil_animation_has_duration(path: impl AsRef<Path>) -> bool {
    let Ok(bytes) = std::fs::read(path.as_ref()) else {
        return false;
    };

    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return gif_frame_durations_ms(&bytes)
            .map(|(durations, _)| durations.len() > 1)
            .unwrap_or(false);
    }

    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        let (durations, _) = webp_frame_durations_ms(&bytes);

        return durations.len() > 1;
    }

    false
}

/// `is_png_animated( file_header_bytes )`
pub fn is_png_animated(file_header_bytes: &[u8]) -> bool {
    get_apng_actl_chunk_data(file_header_bytes)
        .map(|actl| get_apng_num_frames(&actl) > 1)
        .unwrap_or(false)
}

/// `get_times_to_play_apng( path )`
pub fn get_times_to_play_apng(path: impl AsRef<Path>) -> u32 {
    let Ok(bytes) = std::fs::read(path.as_ref()) else {
        return 0;
    };

    let header = &bytes[..bytes.len().min(256)];

    get_apng_actl_chunk_data(header)
        .and_then(|actl| actl.get(4..8).and_then(u32_be))
        .unwrap_or(0)
}

/// `get_times_to_play_pil_animation( path, human_file_description = None )`
pub fn get_times_to_play_pil_animation(
    path: impl AsRef<Path>,
    _human_file_description: Option<&str>,
) -> u32 {
    let Ok(bytes) = std::fs::read(path.as_ref()) else {
        return 1;
    };

    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return gif_frame_durations_ms(&bytes)
            .map(|(_, times_to_play)| times_to_play)
            .unwrap_or(1);
    }

    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        let (_, times_to_play) = webp_frame_durations_ms(&bytes);

        return times_to_play;
    }

    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
        chunk.extend_from_slice(name);
        chunk.extend_from_slice(data);
        chunk.extend_from_slice(&[0; 4]); // CRC is not read by our parser.

        chunk
    }

    fn apng_header(num_frames: u32, times_to_play: u32, delays: &[(u16, u16)]) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();

        let mut actl = Vec::new();
        actl.extend_from_slice(&num_frames.to_be_bytes());
        actl.extend_from_slice(&times_to_play.to_be_bytes());
        bytes.extend_from_slice(&png_chunk(b"acTL", &actl));

        for (numerator, denominator) in delays {
            let mut fctl = vec![0_u8; 24];
            fctl[20..22].copy_from_slice(&numerator.to_be_bytes());
            fctl[22..24].copy_from_slice(&denominator.to_be_bytes());
            bytes.extend_from_slice(&png_chunk(b"fcTL", &fctl));
        }

        bytes
    }

    fn minimal_gif(frame_count: usize, delay: u16) -> Vec<u8> {
        let mut bytes = b"GIF89a".to_vec();
        // Logical screen: width 1, height 1, no global colour table.
        bytes.extend_from_slice(&[1, 0, 1, 0, 0x00, 0, 0]);

        // NETSCAPE loop extension.
        bytes.extend_from_slice(&[0x21, 0xFF, 11]);
        bytes.extend_from_slice(b"NETSCAPE2.0");
        bytes.extend_from_slice(&[3, 1, 0, 0, 0]);

        for _ in 0..frame_count {
            bytes.extend_from_slice(&[0x21, 0xF9, 4, 0, delay as u8, (delay >> 8) as u8, 0, 0]);
            bytes.extend_from_slice(&[0x2C, 0, 0, 0, 0, 1, 0, 1, 0, 0x00, 2]);
            // Minimal LZW sub-block and terminator.
            bytes.extend_from_slice(&[2, 0x44, 0x01, 0]);
        }

        bytes.push(0x3B);

        bytes
    }

    fn minimal_webp() -> Vec<u8> {
        let mut bytes = b"RIFF".to_vec();
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(b"WEBP");

        let mut anim = vec![0_u8; 6];
        anim[4..6].copy_from_slice(&3_u16.to_le_bytes());
        bytes.extend_from_slice(b"ANIM");
        bytes.extend_from_slice(&(anim.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&anim);

        for duration in [40_u32, 50] {
            let mut anmf = vec![0_u8; 16];
            anmf[12..15].copy_from_slice(&duration.to_le_bytes()[..3]);
            bytes.extend_from_slice(b"ANMF");
            bytes.extend_from_slice(&(anmf.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&anmf);
        }

        bytes
    }

    #[test]
    fn apng_actl_and_fctl_are_read() {
        let bytes = apng_header(2, 0, &[(1, 10), (2, 10)]);

        assert!(is_png_animated(&bytes));
        assert_eq!(
            get_apng_actl_chunk_data(&bytes).map(|data| get_apng_num_frames(&data)),
            Some(2)
        );
        assert_eq!(get_apng_duration_ms(&bytes), 300);
    }

    #[test]
    fn a_single_frame_apng_is_not_animated() {
        let bytes = apng_header(1, 0, &[(1, 10)]);

        assert!(!is_png_animated(&bytes));
    }

    #[test]
    fn gif_delays_and_loop_count_are_read() {
        let bytes = minimal_gif(2, 10);

        let (durations, times_to_play) = gif_frame_durations_ms(&bytes).unwrap();

        assert_eq!(durations.len(), 2);
        assert_eq!(durations[0], 100);
        assert_eq!(times_to_play, 0);
    }

    #[test]
    fn webp_anim_and_anmf_are_read() {
        let bytes = minimal_webp();

        let (durations, times_to_play) = webp_frame_durations_ms(&bytes);

        assert_eq!(durations, vec![40, 50]);
        assert_eq!(times_to_play, 3);
    }

    #[test]
    fn zero_duration_animated_webp_uses_the_fallback() {
        let mut bytes = minimal_webp();
        // Rewrite the first ANMF duration.
        let anmf = bytes
            .windows(4)
            .position(|window| window == b"ANMF")
            .unwrap();
        bytes[anmf + 8 + 12..anmf + 8 + 15].copy_from_slice(&[0, 0, 0]);

        let (durations, _) = webp_frame_durations_ms(&bytes);

        assert_eq!(durations[0], FALLBACK_FRAME_DURATION_MS);
    }
}
