//! Rust port of `hydrus/core/files/images/HydrusImageICCProfilesTagSlop.py`.
//!
//! These are the small ICC tag writers used by
//! [`crate::files::images::hydrus_image_icc_profiles`]. All multi-byte integers and floats are
//! big-endian, as the ICC spec requires.

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::files::images::hydrus_image_math_slop::{float_to_s15_fixed16, Matrix3};

fn pad_to_four(bytes: &mut Vec<u8>) {
    let padding = (4 - (bytes.len() % 4)) % 4;

    bytes.extend(std::iter::repeat_n(0_u8, padding));
}

fn append_f32_be(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn append_i32_be(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn append_u16_be(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn append_u32_be(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn sf32_tag(matrix: Matrix3) -> Vec<u8> {
    let mut tag = Vec::with_capacity(44);

    tag.extend_from_slice(b"sf32");
    tag.extend_from_slice(&[0; 4]);

    for row in matrix {
        for value in row {
            append_f32_be(&mut tag, value as f32);
        }
    }

    tag
}

/// `make_chad_tag_arbitrary_to_D50( src_white_xy )`
pub fn make_chad_tag_arbitrary_to_d50(src_white_xy: (f64, f64)) -> Vec<u8> {
    sf32_tag(crate::files::images::hydrus_image_math_slop::generate_bradford_adaptation_matrix(src_white_xy))
}

/// `make_chad_tag_D50()`
pub fn make_chad_tag_d50() -> Vec<u8> {
    make_chad_tag_identity()
}

/// `make_chad_tag_D65_to_D50()`
pub fn make_chad_tag_d65_to_d50() -> Vec<u8> {
    sf32_tag([
        [0.9555766, -0.0230393, 0.0631636],
        [-0.0282895, 1.0099416, 0.0210077],
        [0.0122982, -0.0204830, 1.3299098],
    ])
}

/// `make_chad_tag_identity()`
pub fn make_chad_tag_identity() -> Vec<u8> {
    sf32_tag([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
}

/// `make_chad_tag_src_to_D50( src_white_xy )`
pub fn make_chad_tag_src_to_d50(src_white_xy: (f64, f64)) -> Vec<u8> {
    make_chad_tag_arbitrary_to_d50(src_white_xy)
}

/// `make_curv_gamma_tag( gamma )`
pub fn make_curv_gamma_tag(gamma: f64) -> Vec<u8> {
    let g_u8f8 = (gamma * 256.0).round().clamp(0.0, 65535.0) as u16;

    let mut tag = Vec::with_capacity(14);

    tag.extend_from_slice(b"curv");
    tag.extend_from_slice(&[0; 4]);
    append_u32_be(&mut tag, 1);
    append_u16_be(&mut tag, g_u8f8);

    pad_to_four(&mut tag);

    tag
}

/// `make_para_curve_tag( gamma )`
pub fn make_para_curve_tag(gamma: f64) -> Vec<u8> {
    let mut tag = Vec::with_capacity(16);

    tag.extend_from_slice(b"para");
    tag.extend_from_slice(&[0; 4]);
    append_u16_be(&mut tag, 0);
    tag.extend_from_slice(&[0; 2]);
    append_i32_be(&mut tag, float_to_s15_fixed16(gamma));

    pad_to_four(&mut tag);

    tag
}

/// `make_para_curve_tag_gamma( gamma )`
pub fn make_para_curve_tag_gamma(gamma: f64) -> Vec<u8> {
    make_para_curve_tag(gamma)
}

/// `make_wtpt_tag_D65()`
///
/// The Python comment says D50, but the values are D65; the code is preserved.
pub fn make_wtpt_tag_d65() -> Vec<u8> {
    pack_xyz_tag(0.95047, 1.0, 1.08883)
}

/// `make_wtpt_tag_D50()`
pub fn make_wtpt_tag_d50() -> Vec<u8> {
    pack_xyz_tag(0.9642, 1.0, 0.8249)
}

/// `make_xyz_matrix_tags( m )`
pub fn make_xyz_matrix_tags(matrix: Matrix3) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    // ICC expects columns to be the XYZ values for the R, G and B primaries.
    (
        pack_xyz_tag(matrix[0][0], matrix[1][0], matrix[2][0]),
        pack_xyz_tag(matrix[0][1], matrix[1][1], matrix[2][1]),
        pack_xyz_tag(matrix[0][2], matrix[1][2], matrix[2][2]),
    )
}

/// `make_desc_tag( text )`
pub fn make_desc_tag(text: &str) -> HydrusResult<Vec<u8>> {
    if !text.is_ascii() {
        return Err(HydrusException::new(
            ErrorKind::Unknown,
            "ICC description text must be ASCII.",
        ));
    }

    let mut string_bytes = text.as_bytes().to_vec();
    string_bytes.push(0);

    let mut tag = Vec::with_capacity(12 + string_bytes.len());

    tag.extend_from_slice(b"desc");
    tag.extend_from_slice(&[0; 4]);
    append_u32_be(&mut tag, string_bytes.len() as u32);
    tag.extend_from_slice(&string_bytes);

    pad_to_four(&mut tag);

    Ok(tag)
}

/// `make_text_tag( text )`
pub fn make_text_tag(text: &str) -> Vec<u8> {
    let mut tag = Vec::with_capacity(8 + text.len());

    tag.extend_from_slice(b"text");
    tag.extend_from_slice(&[0; 4]);

    for character in text.chars() {
        tag.push(if character.is_ascii() { character as u8 } else { b'?' });
    }

    pad_to_four(&mut tag);

    tag
}

/// `pack_xyz_tag( x, y, z )`
pub fn pack_xyz_tag(x: f64, y: f64, z: f64) -> Vec<u8> {
    let mut tag = Vec::with_capacity(20);

    tag.extend_from_slice(b"XYZ ");
    tag.extend_from_slice(&[0; 4]);
    append_i32_be(&mut tag, float_to_s15_fixed16(x));
    append_i32_be(&mut tag, float_to_s15_fixed16(y));
    append_i32_be(&mut tag, float_to_s15_fixed16(z));

    tag
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_chad_is_a_big_endian_sf32_tag() {
        let tag = make_chad_tag_identity();

        assert_eq!(&tag[..4], b"sf32");
        assert_eq!(tag.len(), 44);
        assert_eq!(f32::from_be_bytes(tag[8..12].try_into().unwrap()), 1.0);
        assert_eq!(f32::from_be_bytes(tag[40..44].try_into().unwrap()), 1.0);
    }

    #[test]
    fn gamma_curve_uses_u8fixed8_and_pads() {
        let tag = make_curv_gamma_tag(2.2);

        assert_eq!(&tag[..4], b"curv");
        assert_eq!(u32::from_be_bytes(tag[8..12].try_into().unwrap()), 1);
        assert_eq!(u16::from_be_bytes(tag[12..14].try_into().unwrap()), 563);
        assert_eq!(tag.len(), 16);
    }

    #[test]
    fn desc_and_text_tags_are_written() {
        let desc = make_desc_tag("hello").unwrap();

        assert_eq!(&desc[..4], b"desc");
        assert_eq!(u32::from_be_bytes(desc[8..12].try_into().unwrap()), 6);

        let text = make_text_tag("hello\u{00e9}");

        assert_eq!(&text[..4], b"text");
        assert_eq!(&text[8..14], b"hello?");
    }

    #[test]
    fn xyz_tags_have_three_fixed_numbers() {
        let tag = pack_xyz_tag(1.0, 0.5, -1.0);

        assert_eq!(&tag[..4], b"XYZ ");
        assert_eq!(i32::from_be_bytes(tag[8..12].try_into().unwrap()), 65536);
        assert_eq!(i32::from_be_bytes(tag[12..16].try_into().unwrap()), 32768);
        assert_eq!(i32::from_be_bytes(tag[16..20].try_into().unwrap()), -65536);
    }
}
