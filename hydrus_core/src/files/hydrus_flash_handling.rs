//! Rust port of `hydrus/core/files/HydrusFlashHandling.py`.
//!
//! Python uses the tiny `hexagonit.swfheader` library to read the SWF header.
//! The port keeps that parser here so no external Python module is required.
//! It supports uncompressed (`FWS`), zlib (`CWS`) and LZMA (`ZWS`) SWFs.

use std::io::Cursor;
use std::path::Path;

use flate2::read::ZlibDecoder;
use std::io::Read;

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// The fields `hexagonit.swfheader.parse` returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwfHeader {
    pub compressed: bool,
    pub version: u8,
    pub size: u32,
    pub xmin: i64,
    pub xmax: i64,
    pub ymin: i64,
    pub ymax: i64,
    pub width: i64,
    pub height: i64,
    pub fps: u16,
    pub frames: u16,
}

fn damaged(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DamagedOrUnusualFile, message)
}

fn read_u16(payload: &[u8], offset: usize) -> HydrusResult<u16> {
    let bytes = payload
        .get(offset..offset + 2)
        .ok_or_else(|| damaged("SWF header is too short!"))?;

    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn decompress_zlib(payload: &[u8]) -> HydrusResult<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(payload);
    let mut output = Vec::new();

    decoder
        .read_to_end(&mut output)
        .map_err(|error| damaged(format!("Could not decompress the zlib SWF payload: {error}")))?;

    Ok(output)
}

fn decompress_lzma(payload: &[u8], size: u32) -> HydrusResult<Vec<u8>> {
    if payload.len() < 9 {
        return Err(damaged("SWF LZMA header is too short!"));
    }

    // The SWF ZWS layout is a four-byte uncompressed size followed by the
    // five-byte LZMA properties. `lzma-rs` expects the usual thirteen-byte
    // .lzma header, so fill the unknown size with 0xFF exactly like the
    // Python helper does.
    let the_five = &payload[4..9];
    let the_rest = &payload[9..];

    let mut lzma_data = Vec::with_capacity(13 + the_rest.len());

    lzma_data.extend_from_slice(the_five);
    lzma_data.extend_from_slice(&[0xFF; 8]);
    lzma_data.extend_from_slice(the_rest);

    let mut input = Cursor::new(lzma_data);
    let mut output = Vec::with_capacity(size as usize);

    lzma_rs::lzma_decompress(&mut input, &mut output)
        .map_err(|error| damaged(format!("Could not decompress the LZMA SWF payload: {error}")))?;

    Ok(output)
}

fn parse_rect(payload: &[u8]) -> HydrusResult<([i64; 4], usize)> {
    let first_byte = *payload
        .first()
        .ok_or_else(|| damaged("SWF RECT is empty!"))?;

    let nbits = usize::from(first_byte >> 3);

    let mut current_byte = usize::from(first_byte);
    let mut bit_cursor = 5usize;
    let mut index = 1usize;

    let mut values = [0i64; 4];

    for value in &mut values {
        let mut result = 0i64;

        for value_bit in (0..nbits).rev() {
            if (current_byte << bit_cursor) & 0x80 != 0 {
                result |= 1i64 << value_bit;
            }

            bit_cursor += 1;

            if bit_cursor > 7 {
                current_byte = usize::from(
                    *payload
                        .get(index)
                        .ok_or_else(|| damaged("SWF RECT is truncated!"))?,
                );

                index += 1;
                bit_cursor = 0;
            }
        }

        *value = result / 20;
    }

    Ok((values, index))
}

/// `hexagonitswfheader.parse`, reduced to the fields Hydrus reads.
pub fn parse_swf_header_bytes(data: &[u8]) -> HydrusResult<SwfHeader> {
    if data.len() < 8 {
        return Err(damaged("SWF file is too short!"));
    }

    let signature = std::str::from_utf8(&data[0..3])
        .map_err(|_| damaged("SWF signature is not valid UTF-8!"))?;

    if !matches!(signature, "FWS" | "CWS" | "ZWS") {
        return Err(damaged(format!("Invalid SWF signature: {signature}")));
    }

    let compressed = signature.starts_with('C') || signature.starts_with('Z');
    let version = data[3];
    let size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

    let payload = match signature {
        "FWS" => data[8..].to_vec(),
        "CWS" => decompress_zlib(&data[8..])?,
        "ZWS" => decompress_lzma(&data[8..], size)?,
        _ => unreachable!(),
    };

    let ([xmin, xmax, ymin, ymax], index) = parse_rect(&payload)?;

    let fps = read_u16(&payload, index)? >> 8;
    let frames = read_u16(&payload, index + 2)?;

    Ok(SwfHeader {
        compressed,
        version,
        size,
        xmin,
        xmax,
        ymin,
        ymax,
        width: xmax - xmin,
        height: ymax - ymin,
        fps,
        frames,
    })
}

/// `hexagonitswfheader.parse( path )`
pub fn parse_swf_header(path: impl AsRef<Path>) -> HydrusResult<SwfHeader> {
    let data = std::fs::read(path.as_ref()).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!(
                "Could not read the SWF file \"{}\": {error}",
                path.as_ref().display()
            ),
        )
    })?;

    parse_swf_header_bytes(&data)
}

/// `get_flash_properties( path )`
///
/// Returns `( ( width, height ), duration_ms, num_frames )`.
pub fn get_flash_properties(path: impl AsRef<Path>) -> HydrusResult<((u64, u64), u64, u32)> {
    let header = parse_swf_header(path)?;

    let width = header.width.unsigned_abs();
    let height = header.height.unsigned_abs();

    let fps = if header.fps == 0 { 1 } else { header.fps };

    let duration_ms = (u64::from(header.frames) * 1000) / u64::from(fps);

    Ok(((width, height), duration_ms, u32::from(header.frames)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_fws_swf() -> Vec<u8> {
        // A 1x1 pixel RECT (20 TWIPS per value) and 12 fps, one frame.
        let mut bits = Vec::new();

        let push_bits = |bits: &mut Vec<bool>, value: u32, count: usize| {
            for bit in (0..count).rev() {
                bits.push((value >> bit) & 1 == 1);
            }
        };

        push_bits(&mut bits, 5, 5); // nbits
        push_bits(&mut bits, 0, 5); // xmin
        push_bits(&mut bits, 20, 5); // xmax
        push_bits(&mut bits, 0, 5); // ymin
        push_bits(&mut bits, 20, 5); // ymax

        let mut rect = vec![0u8; 4];

        for (index, bit) in bits.iter().enumerate() {
            if *bit {
                rect[index / 8] |= 1 << (7 - (index % 8));
            }
        }

        let mut payload = rect;
        payload.extend_from_slice(&(12u16 << 8).to_le_bytes()); // fps
        payload.extend_from_slice(&1u16.to_le_bytes()); // frames

        let mut swf = Vec::new();
        swf.extend_from_slice(b"FWS");
        swf.push(9);
        swf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        swf.extend_from_slice(&payload);

        swf
    }

    #[test]
    fn a_minimal_fws_swf_has_dimensions_fps_and_frames() {
        let header = parse_swf_header_bytes(&minimal_fws_swf()).unwrap();

        assert!(!header.compressed);
        assert_eq!(header.version, 9);
        assert_eq!((header.width, header.height), (1, 1));
        assert_eq!(header.fps, 12);
        assert_eq!(header.frames, 1);
    }

    #[test]
    fn properties_use_absolute_dimensions_and_guard_zero_fps() {
        let mut swf = minimal_fws_swf();

        // Zero the fps bytes in the FWS payload.
        let fps_offset = 3 + 1 + 4 + 4;
        swf[fps_offset] = 0;
        swf[fps_offset + 1] = 0;

        let unique = format!(
            "hydrus_swf_{}_{}.swf",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );

        let path = std::env::temp_dir().join(unique);

        std::fs::write(&path, &swf).unwrap();

        let ((width, height), duration_ms, frames) = get_flash_properties(&path).unwrap();

        assert_eq!((width, height), (1, 1));
        assert_eq!(frames, 1);
        assert_eq!(duration_ms, 1000);

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn invalid_signatures_are_damaged_files() {
        let error = parse_swf_header_bytes(b"XYZ\x09\x00\x00\x00\x00").unwrap_err();

        assert!(error.is_a(ErrorKind::DamagedOrUnusualFile));
        assert!(error.to_string().contains("Invalid SWF signature"));
    }
}
