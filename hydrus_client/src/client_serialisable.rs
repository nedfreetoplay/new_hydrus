//! Serialised-PNG transport from `hydrus/client/ClientSerialisable.py`.
//!
//! The payload lives in the grayscale pixels below a small display header:
//! two bytes of header height, then a four-byte big-endian length, then the
//! zlib/LZ4-compressed serialisable string.

use std::path::Path;

use hydrus_core::hydrus_compression;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::hydrus_serialisable::SerialisableObject;
use image::{GrayImage, Luma};

pub const DEFAULT_WIDTH: u32 = 512;

/// `DumpToPNG` for a serialisable string.
pub fn dump_string_to_png(path: impl AsRef<Path>, payload: &str, width: u32) -> HydrusResult<()> {
    let compressed = hydrus_compression::compress_string_to_bytes(payload)?;

    dump_compressed_bytes_to_png(path, &compressed, width)
}

pub fn dump_object_to_png(
    path: impl AsRef<Path>,
    object: &dyn SerialisableObject,
    width: u32,
) -> HydrusResult<()> {
    dump_string_to_png(path, &object.dump_to_string(), width)
}

pub fn dump_bytes_to_png(path: impl AsRef<Path>, payload: &[u8], width: u32) -> HydrusResult<()> {
    let compressed = hydrus_compression::compress_bytes_to_bytes(payload)?;

    dump_compressed_bytes_to_png(path, &compressed, width)
}

fn dump_compressed_bytes_to_png(
    path: impl AsRef<Path>,
    compressed: &[u8],
    width: u32,
) -> HydrusResult<()> {
    if width == 0 {
        return Err(HydrusException::new(
            ErrorKind::Serialisation,
            "PNG width must be positive",
        ));
    }

    let width = width as usize;
    let payload_length = compressed.len();
    let payload_rows = (4 + payload_length).div_ceil(width);
    // One row is the visual header. Its first two bytes store the header height,
    // which the loader uses to find the payload.
    let top_height = 1usize;
    let total_rows = top_height + payload_rows;

    let mut pixels = vec![0u8; width * total_rows];

    pixels[0..2].copy_from_slice(&(top_height as u16).to_be_bytes());
    let payload_start = width * top_height;
    pixels[payload_start..payload_start + 4]
        .copy_from_slice(&(payload_length as u32).to_be_bytes());
    pixels[payload_start + 4..payload_start + 4 + payload_length].copy_from_slice(compressed);

    let image: GrayImage = GrayImage::from_raw(width as u32, total_rows as u32, pixels)
        .ok_or_else(|| HydrusException::new(ErrorKind::Serialisation, "could not build PNG"))?;

    image.save(path.as_ref()).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!("Could not save serialised PNG: {error}"),
        )
    })
}

/// `LoadFromPNG` — returns the compressed payload bytes.
pub fn load_compressed_bytes_from_png(path: impl AsRef<Path>) -> HydrusResult<Vec<u8>> {
    let image = image::open(path.as_ref())
        .map_err(|error| {
            HydrusException::new(
                ErrorKind::FileMissing,
                format!("Could not read serialised PNG: {error}"),
            )
        })?
        .to_luma8();

    let width = image.width() as usize;
    let bytes = image.into_raw();

    if width == 0 || bytes.len() < 2 {
        return Err(HydrusException::new(
            ErrorKind::Serialisation,
            "serialised PNG has no header",
        ));
    }

    let top_height = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
    let payload_start = width * top_height;

    if bytes.len() < payload_start + 4 {
        return Err(HydrusException::new(
            ErrorKind::Serialisation,
            "serialised PNG header is truncated",
        ));
    }

    let payload_length = u32::from_be_bytes([
        bytes[payload_start],
        bytes[payload_start + 1],
        bytes[payload_start + 2],
        bytes[payload_start + 3],
    ]) as usize;

    let payload_end = payload_start + 4 + payload_length;

    if bytes.len() < payload_end {
        return Err(HydrusException::new(
            ErrorKind::Serialisation,
            "serialised PNG payload is truncated",
        ));
    }

    Ok(bytes[payload_start + 4..payload_end].to_vec())
}

/// `LoadStringFromPNG`
pub fn load_string_from_png(path: impl AsRef<Path>) -> HydrusResult<String> {
    let payload = load_compressed_bytes_from_png(path)?;

    if let Ok(decompressed) = hydrus_compression::decompress_bytes_to_string(&payload) {
        return Ok(decompressed);
    }

    // Older payloads were stored uncompressed.
    String::from_utf8(payload)
        .map_err(|error| HydrusException::new(ErrorKind::Serialisation, error.to_string()))
}

pub fn load_bytes_from_png(path: impl AsRef<Path>) -> HydrusResult<Vec<u8>> {
    let payload = load_compressed_bytes_from_png(path)?;

    hydrus_compression::decompress_bytes_to_bytes(&payload).or(Ok(payload))
}

/// A visual header writer, kept separate so later GUI work can draw the title.
pub fn make_blank_image(width: u32, height: u32) -> GrayImage {
    GrayImage::from_pixel(width.max(1), height.max(1), Luma([0u8]))
}

#[cfg(test)]
mod tests;
