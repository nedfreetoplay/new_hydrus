//! Rust port of `hydrus/core/files/images/HydrusImageICCProfiles.py`.
//!
//! The Python module synthesises a small ICC v2 display profile from PNG gamma
//! and chromaticity chunks. The generated profile is deliberately modest: a
//! description, media white point, chromatic adaptation matrix, RGB primary
//! matrix, and one gamma curve shared by all three channels.

use chrono::{Datelike, Timelike};

use crate::hydrus_date_time;
use crate::hydrus_image_icc_profiles_tag_slop as tag_slop;
use crate::hydrus_image_math_slop::{float_to_s15_fixed16, Matrix3};

const ICC_HEADER_SIZE: usize = 128;

fn append_u16_be(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn append_u32_be(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn append_i32_be(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn append_u64_be(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

/// `make_icc_header( profile_size )`
pub fn make_icc_header(profile_size: u32) -> Vec<u8> {
    let now = hydrus_date_time::now_utc();

    let mut header = Vec::with_capacity(ICC_HEADER_SIZE);

    append_u32_be(&mut header, profile_size);
    header.extend_from_slice(b"lcms");
    header.extend_from_slice(&[0x02, 0x10, 0x00, 0x00]);
    header.extend_from_slice(b"mntr");
    header.extend_from_slice(b"RGB ");
    header.extend_from_slice(b"XYZ ");

    append_u16_be(&mut header, now.year() as u16);
    append_u16_be(&mut header, now.month() as u16);
    append_u16_be(&mut header, now.day() as u16);
    append_u16_be(&mut header, now.hour() as u16);
    append_u16_be(&mut header, now.minute() as u16);
    append_u16_be(&mut header, now.second() as u16);

    header.extend_from_slice(b"acsp");
    header.extend_from_slice(b"    ");
    append_u32_be(&mut header, 0); // profile flags
    append_u32_be(&mut header, 0); // device manufacturer
    append_u32_be(&mut header, 0); // device model
    append_u64_be(&mut header, 0); // device attributes
    append_u32_be(&mut header, 0); // rendering intent
    append_i32_be(&mut header, float_to_s15_fixed16(0.9642));
    append_i32_be(&mut header, float_to_s15_fixed16(1.0));
    append_i32_be(&mut header, float_to_s15_fixed16(0.8249));
    header.extend_from_slice(b"none");
    header.extend_from_slice(&[0; 16]); // profile ID
    header.extend_from_slice(&[0; 28]); // reserved

    debug_assert_eq!(header.len(), ICC_HEADER_SIZE);

    header
}

fn pad_tag_data(data: &[u8]) -> Vec<u8> {
    let mut padded = data.to_vec();

    let padding = (4 - (padded.len() % 4)) % 4;

    padded.extend(std::iter::repeat_n(0_u8, padding));

    padded
}

/// `make_gamma_and_chromaticity_icc_profile( gamma, chromaticity_xyz_matrix_D50 )`
pub fn make_gamma_and_chromaticity_icc_profile(
    gamma: f64,
    chromaticity_xyz_matrix_d50: Matrix3,
) -> Vec<u8> {
    let desc = tag_slop::make_desc_tag("iccp")
        .expect("the hard-coded ICC profile description is ASCII");
    let cprt = tag_slop::make_text_tag("Generated from PNG gamma/chromaticities");

    let (r_xyz, g_xyz, b_xyz) =
        tag_slop::make_xyz_matrix_tags(chromaticity_xyz_matrix_d50);

    let trc = tag_slop::make_curv_gamma_tag(gamma);

    let tag_items: [(&[u8; 4], Vec<u8>); 10] = [
        (b"desc", desc),
        (b"cprt", cprt),
        (b"wtpt", tag_slop::make_wtpt_tag_d50()),
        (b"chad", tag_slop::make_chad_tag_identity()),
        (b"rXYZ", r_xyz),
        (b"gXYZ", g_xyz),
        (b"bXYZ", b_xyz),
        (b"rTRC", trc.clone()),
        (b"gTRC", trc.clone()),
        (b"bTRC", trc),
    ];

    let tag_count = tag_items.len();
    let tag_table_size = 4 + tag_count * 12;
    let mut offset = ICC_HEADER_SIZE + tag_table_size;

    let mut tag_table = Vec::with_capacity(tag_table_size);
    append_u32_be(&mut tag_table, tag_count as u32);

    let mut data_blocks = Vec::with_capacity(tag_count);

    for (signature, data) in tag_items {
        let padded = pad_tag_data(&data);

        tag_table.extend_from_slice(signature);
        append_u32_be(&mut tag_table, offset as u32);
        append_u32_be(&mut tag_table, data.len() as u32);

        offset += padded.len();
        data_blocks.push(padded);
    }

    let mut profile_body = tag_table;

    for block in data_blocks {
        profile_body.extend_from_slice(&block);
    }

    let mut total_size = ICC_HEADER_SIZE + profile_body.len();
    let final_padding = (4 - (total_size % 4)) % 4;
    total_size += final_padding;

    let mut profile = make_icc_header(total_size as u32);
    profile.extend_from_slice(&profile_body);
    profile.extend(std::iter::repeat_n(0_u8, final_padding));

    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_is_exactly_128_bytes() {
        let header = make_icc_header(1234);

        assert_eq!(header.len(), 128);
        assert_eq!(&header[..4], &1234_u32.to_be_bytes());
        assert_eq!(&header[36..40], b"acsp");
        assert_eq!(&header[12..16], b"mntr");
        assert_eq!(&header[16..20], b"RGB ");
        assert_eq!(&header[20..24], b"XYZ ");
    }

    #[test]
    fn profile_has_a_tag_table_and_aligned_size() {
        let profile = make_gamma_and_chromaticity_icc_profile(
            2.2,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        );

        let size = u32::from_be_bytes(profile[..4].try_into().unwrap()) as usize;

        assert_eq!(size, profile.len());
        assert_eq!(profile.len() % 4, 0);
        assert_eq!(&profile[128..132], &10_u32.to_be_bytes());
        assert_eq!(&profile[132..136], b"desc");
    }
}
