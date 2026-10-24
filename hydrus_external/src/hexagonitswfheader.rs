//! Rust port of `hydrus_python/hydrus/external/hexagonitswfheader.py`.
//!
//! The parser itself lives in `hydrus_core`; this module exposes it under the
//! same module name as the Python external package and provides both path and
//! bytes entry points.

use std::path::Path;

use hydrus_core::hydrus_exceptions::HydrusResult;

pub use hydrus_core::files::hydrus_flash_handling::{
    SwfHeader, get_flash_properties, parse_swf_header_bytes,
};

/// `parse( path )`
pub fn parse_path(path: impl AsRef<Path>) -> HydrusResult<SwfHeader> {
    hydrus_core::files::hydrus_flash_handling::parse_swf_header(path)
}

/// `parse( bytes )`
pub fn parse_bytes(data: &[u8]) -> HydrusResult<SwfHeader> {
    parse_swf_header_bytes(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_fws() -> Vec<u8> {
        let mut bits = Vec::new();

        let push_bits = |bits: &mut Vec<bool>, value: u32, count: usize| {
            for bit in (0..count).rev() {
                bits.push((value >> bit) & 1 == 1);
            }
        };

        push_bits(&mut bits, 15, 5);
        push_bits(&mut bits, 0, 15);
        push_bits(&mut bits, 20, 15);
        push_bits(&mut bits, 0, 15);
        push_bits(&mut bits, 20, 15);

        while bits.len() % 8 != 0 {
            bits.push(false);
        }

        let mut rect = vec![0_u8; bits.len() / 8];

        for (index, bit) in bits.iter().enumerate() {
            if *bit {
                rect[index / 8] |= 1 << (7 - (index % 8));
            }
        }

        let mut payload = rect;
        payload.extend_from_slice(&(12_u16 << 8).to_le_bytes());
        payload.extend_from_slice(&1_u16.to_le_bytes());

        let mut swf = Vec::new();
        swf.extend_from_slice(b"FWS");
        swf.push(9);
        swf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        swf.extend_from_slice(&payload);

        swf
    }

    #[test]
    fn minimal_fws_header_is_parsed() {
        let header = parse_bytes(&minimal_fws()).unwrap();

        assert!(!header.compressed);
        assert_eq!(header.version, 9);
        assert_eq!((header.width, header.height), (1, 1));
        assert_eq!(header.fps, 12);
        assert_eq!(header.frames, 1);
    }
}
