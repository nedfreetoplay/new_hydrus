//! Rust port of `hydrus/core/HydrusCompression.py`.
//!
//! zlib maps to [`flate2`]; the optional `lz4.block` fast path maps to
//! [`lz4_flex`]. Python's `lz4.block.compress` defaults to
//! `store_size=True`, which prepends the little-endian uncompressed size;
//! `lz4_flex::block::compress_prepend_size` writes the same header, so the
//! bytes remain compatible.
//!
//! Python's `LZ4_OK` depends on whether the package imported. The Rust port
//! always builds the dependency, so [`LZ4_OK`] is `true`.

use std::io::{Read as _, Write as _};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

pub const LZ4_OK: bool = true;

fn decompression_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::Parse, message)
}

/// `compress_bytes_to_bytes( obj_bytes )`
pub fn compress_bytes_to_bytes(obj_bytes: &[u8]) -> HydrusResult<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());

    encoder
        .write_all(obj_bytes)
        .map_err(|e| decompression_error(format!("could not compress bytes: {e}")))?;

    encoder
        .finish()
        .map_err(|e| decompression_error(format!("could not compress bytes: {e}")))
}

/// `compress_fast_bytes_to_bytes( obj_bytes )`
pub fn compress_fast_bytes_to_bytes(obj_bytes: &[u8]) -> Vec<u8> {
    if LZ4_OK {
        lz4_flex::block::compress_prepend_size(obj_bytes)
    } else {
        obj_bytes.to_vec()
    }
}

/// `compress_string_to_bytes( obj_string )`
pub fn compress_string_to_bytes(obj_string: &str) -> HydrusResult<Vec<u8>> {
    compress_bytes_to_bytes(obj_string.as_bytes())
}

/// `decompress_bytes_to_bytes( compressed_bytes )`
pub fn decompress_bytes_to_bytes(compressed_bytes: &[u8]) -> HydrusResult<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(compressed_bytes);

    let mut obj_bytes = Vec::new();

    if decoder.read_to_end(&mut obj_bytes).is_ok() {
        return Ok(obj_bytes);
    }

    if LZ4_OK {
        lz4_flex::block::decompress_size_prepended(compressed_bytes)
            .map_err(|e| decompression_error(format!("could not decompress bytes: {e}")))
    } else {
        Err(decompression_error("could not decompress bytes"))
    }
}

/// `decompress_bytes_to_string( compressed_bytes )`
pub fn decompress_bytes_to_string(compressed_bytes: &[u8]) -> HydrusResult<String> {
    let obj_bytes = decompress_bytes_to_bytes(compressed_bytes)?;

    String::from_utf8(obj_bytes)
        .map_err(|e| decompression_error(format!("decompressed bytes were not UTF-8: {e}")))
}

/// `decompress_fast_bytes_to_bytes( compressed_bytes )`
pub fn decompress_fast_bytes_to_bytes(compressed_bytes: &[u8]) -> HydrusResult<Vec<u8>> {
    if LZ4_OK {
        lz4_flex::block::decompress_size_prepended(compressed_bytes)
            .map_err(|e| decompression_error(format!("could not decompress bytes: {e}")))
    } else {
        Ok(compressed_bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zlib_round_trip() {
        let data = b"hello hydrus, hello hydrus, hello hydrus";

        let compressed = compress_bytes_to_bytes(data).unwrap();

        assert_ne!(compressed, data);
        assert_eq!(decompress_bytes_to_bytes(&compressed).unwrap(), data);
    }

    #[test]
    fn zlib_string_round_trip() {
        let text = "tag one, tag two \u{1f600}";

        let compressed = compress_string_to_bytes(text).unwrap();

        assert_eq!(decompress_bytes_to_string(&compressed).unwrap(), text);
    }

    #[test]
    fn fast_round_trip() {
        let data = b"some bytes that should survive lz4";

        let compressed = compress_fast_bytes_to_bytes(data);

        assert_eq!(decompress_fast_bytes_to_bytes(&compressed).unwrap(), data);
    }

    #[test]
    fn slow_decompressor_falls_back_to_lz4() {
        let data = b"fallback works";

        let compressed = compress_fast_bytes_to_bytes(data);

        assert_eq!(decompress_bytes_to_bytes(&compressed).unwrap(), data);
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(decompress_bytes_to_bytes(b"not compressed at all").is_err());
    }
}
