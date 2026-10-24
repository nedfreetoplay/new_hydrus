//! Rust port of `hydrus/core/files/images/HydrusBlurhash.py`.
//!
//! The Python module wraps the external `blurhash` package and uses numpy for
//! colour conversion. This port includes a small pure-Rust base83/DCT
//! implementation, so the core crate does not need a dependency on an external
//! BlurHash library.

use std::f64::consts::PI;

use image::{DynamicImage, RgbImage};

use crate::files::images::hydrus_image_handling;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

const BASE83_ALPHABET: &[u8; 83] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz#$%*+,-.:;=?@[]^_{|}~";

const CUTOFF_DIMENSION: u32 = 100;

fn damaged(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DamagedOrUnusualFile, message)
}

fn base83_index(character: u8) -> Option<u32> {
    BASE83_ALPHABET
        .iter()
        .position(|candidate| *candidate == character)
        .map(|index| index as u32)
}

/// `external_blurhash.base83_decode( value )`
pub fn base83_decode(value: &str) -> HydrusResult<u32> {
    let mut result = 0_u32;

    for byte in value.bytes() {
        let digit = base83_index(byte)
            .ok_or_else(|| damaged(format!("Invalid BlurHash base83 character: {byte}")))?;

        result = result
            .checked_mul(83)
            .and_then(|result| result.checked_add(digit))
            .ok_or_else(|| damaged("BlurHash base83 value overflowed."))?;
    }

    Ok(result)
}

/// `external_blurhash.base83_encode( value, length )`
pub fn base83_encode(mut value: u32, length: usize) -> String {
    let mut output = vec![b'0'; length];

    for index in (0..length).rev() {
        output[index] = BASE83_ALPHABET[(value % 83) as usize];
        value /= 83;
    }

    String::from_utf8(output).expect("the base83 alphabet is valid UTF-8")
}

/// `rgb_to_hsl( r, g, b )`
pub fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let r = r / 255.0;
    let g = g / 255.0;
    let b = b / 255.0;

    let max_c = r.max(g).max(b);
    let min_c = r.min(g).min(b);
    let delta = max_c - min_c;

    let lightness = (max_c + min_c) / 2.0;

    let saturation = if delta == 0.0 {
        0.0
    } else if lightness > 0.5 {
        delta / (2.0 - max_c - min_c)
    } else {
        delta / (max_c + min_c)
    };

    let hue = if delta == 0.0 {
        0.0
    } else if max_c == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max_c == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };

    (hue * 60.0, saturation, lightness)
}

/// `rgb_to_xyz( r, g, b )`
pub fn rgb_to_xyz(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    fn gamma_correct(value: f64) -> f64 {
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    let r = gamma_correct(r / 255.0);
    let g = gamma_correct(g / 255.0);
    let b = gamma_correct(b / 255.0);

    let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
    let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
    let z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;

    (x, y, z)
}

/// `xyz_to_lab( x, y, z )`
pub fn xyz_to_lab(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    fn f(value: f64) -> f64 {
        if value > 0.008856 {
            value.powf(1.0 / 3.0)
        } else {
            (7.787 * value) + (16.0 / 116.0)
        }
    }

    let fx = f(x / 0.95047);
    let fy = f(y / 1.0);
    let fz = f(z / 1.08883);

    let lightness = (116.0 * fy) - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);

    (lightness, a, b)
}

/// `convert_blur_hash_to_sortable_chromatic_magnitude( blurhash, reverse )`
pub fn convert_blur_hash_to_sortable_chromatic_magnitude(
    blurhash: &str,
    _reverse: bool,
) -> HydrusResult<(f64, f64)> {
    let (r, g, b) = get_average_colour_from_blur_hash(blurhash)?;
    let (x, y, z) = rgb_to_xyz(f64::from(r), f64::from(g), f64::from(b));
    let (lightness, a, b) = xyz_to_lab(x, y, z);

    Ok(((a * a) + (b * b), lightness))
}

/// `convert_blur_hash_to_sortable_blue_yellow( blurhash, reverse )`
pub fn convert_blur_hash_to_sortable_blue_yellow(
    blurhash: &str,
    _reverse: bool,
) -> HydrusResult<(f64, f64)> {
    let (r, g, b) = get_average_colour_from_blur_hash(blurhash)?;
    let (x, y, z) = rgb_to_xyz(f64::from(r), f64::from(g), f64::from(b));
    let (lightness, _, blue_yellow) = xyz_to_lab(x, y, z);

    Ok((blue_yellow, -lightness))
}

/// `convert_blur_hash_to_sortable_green_red( blurhash, reverse )`
pub fn convert_blur_hash_to_sortable_green_red(
    blurhash: &str,
    _reverse: bool,
) -> HydrusResult<(f64, f64)> {
    let (r, g, b) = get_average_colour_from_blur_hash(blurhash)?;
    let (x, y, z) = rgb_to_xyz(f64::from(r), f64::from(g), f64::from(b));
    let (lightness, green_red, _) = xyz_to_lab(x, y, z);

    Ok((green_red, -lightness))
}

/// `convert_blur_hash_to_sortable_hue( blurhash, reverse )`
pub fn convert_blur_hash_to_sortable_hue(
    blurhash: &str,
    reverse: bool,
) -> HydrusResult<(f64, f64, f64)> {
    let (r, g, b) = get_average_colour_from_blur_hash(blurhash)?;
    let (hue, saturation, _) = rgb_to_hsl(f64::from(r), f64::from(g), f64::from(b));

    let initial = if saturation < 0.03 {
        if reverse { -1.0 } else { 1.0 }
    } else {
        0.0
    };

    Ok((initial, hue, -saturation))
}

/// `convert_blur_hash_to_sortable_lightness( blurhash, reverse )`
pub fn convert_blur_hash_to_sortable_lightness(
    blurhash: &str,
    _reverse: bool,
) -> HydrusResult<(f64, f64)> {
    let (r, g, b) = get_average_colour_from_blur_hash(blurhash)?;
    let (x, y, z) = rgb_to_xyz(f64::from(r), f64::from(g), f64::from(b));
    let (lightness, a, b) = xyz_to_lab(x, y, z);

    Ok((lightness, (a * a) + (b * b)))
}

/// `get_average_colour_from_blur_hash( blurhash )`
///
/// The second through sixth base83 characters are the DC component, which is the
/// image's average colour.
pub fn get_average_colour_from_blur_hash(blurhash: &str) -> HydrusResult<(u8, u8, u8)> {
    if !blurhash.is_ascii() {
        return Err(damaged("BlurHash must be ASCII base83 text."));
    }

    if blurhash.len() < 6 {
        return Err(damaged(
            "BlurHash is too short to contain an average colour.",
        ));
    }

    let average_colour_int = base83_decode(&blurhash[2..6])?;

    Ok((
        ((average_colour_int >> 16) & 0xFF) as u8,
        ((average_colour_int >> 8) & 0xFF) as u8,
        (average_colour_int & 0xFF) as u8,
    ))
}

fn srgb_to_linear(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f64) -> f64 {
    if value <= 0.0031308 {
        12.92 * value
    } else {
        (1.055 * value.powf(1.0 / 2.4)) - 0.055
    }
}

fn sign_pow(value: f64, exponent: f64) -> f64 {
    value.abs().powf(exponent).copysign(value)
}

/// `get_blur_hash_from_numpy( numpy_image )`
pub fn get_blur_hash_from_numpy(numpy_image: &DynamicImage) -> String {
    let (media_width, media_height) = (numpy_image.width(), numpy_image.height());

    if media_width == 0 || media_height == 0 {
        return String::new();
    }

    let ratio = f64::from(media_width) / f64::from(media_height);

    let (components_x, components_y) = if ratio > 4.0 / 3.0 {
        (5_u32, 3_u32)
    } else if ratio < 3.0 / 4.0 {
        (3_u32, 5_u32)
    } else {
        (4_u32, 4_u32)
    };

    let image = if media_width > CUTOFF_DIMENSION || media_height > CUTOFF_DIMENSION {
        hydrus_image_handling::resize_image(
            numpy_image,
            (CUTOFF_DIMENSION, CUTOFF_DIMENSION),
            Some(image::imageops::FilterType::Triangle),
        )
    } else {
        numpy_image.clone()
    };

    let rgb = image.to_rgb8();
    let width = rgb.width().max(1);
    let height = rgb.height().max(1);

    let mut factors: Vec<Vec<[f64; 3]>> =
        vec![vec![[0.0; 3]; components_x as usize]; components_y as usize];

    for j in 0..components_y {
        for i in 0..components_x {
            let normalisation = if i == 0 && j == 0 { 1.0 } else { 2.0 };
            let mut factor = [0.0_f64; 3];

            for y in 0..height {
                let y_basis = (PI * f64::from(y) * f64::from(j) / f64::from(height)).cos();

                for x in 0..width {
                    let basis = (PI * f64::from(x) * f64::from(i) / f64::from(width)).cos()
                        * y_basis
                        * normalisation
                        / (f64::from(width) * f64::from(height));

                    let [r, g, b] = rgb.get_pixel(x, y).0;

                    factor[0] += srgb_to_linear(f64::from(r) / 255.0) * basis;
                    factor[1] += srgb_to_linear(f64::from(g) / 255.0) * basis;
                    factor[2] += srgb_to_linear(f64::from(b) / 255.0) * basis;
                }
            }

            factors[j as usize][i as usize] = factor;
        }
    }

    let dc = factors[0][0];

    let mut maximum_value = 0.0_f64;

    for (j, row) in factors.iter().enumerate() {
        for (i, factor) in row.iter().enumerate() {
            if i == 0 && j == 0 {
                continue;
            }

            for channel in factor {
                maximum_value = maximum_value.max(channel.abs());
            }
        }
    }

    let quantised_maximum_value = ((maximum_value * 166.0) - 0.5).floor().clamp(0.0, 82.0) as u32;
    let maximum_value = f64::from(quantised_maximum_value + 1) / 166.0;

    let linear_r = linear_to_srgb(dc[0].clamp(0.0, 1.0));
    let linear_g = linear_to_srgb(dc[1].clamp(0.0, 1.0));
    let linear_b = linear_to_srgb(dc[2].clamp(0.0, 1.0));

    let rounded_r = (linear_r * 255.0).round().clamp(0.0, 255.0) as u32;
    let rounded_g = (linear_g * 255.0).round().clamp(0.0, 255.0) as u32;
    let rounded_b = (linear_b * 255.0).round().clamp(0.0, 255.0) as u32;

    let dc_value = (rounded_r << 16) + (rounded_g << 8) + rounded_b;

    let size_flag = (components_x - 1) + (components_y * 9);

    let mut hash = base83_encode(size_flag, 1);
    hash.push_str(&base83_encode(quantised_maximum_value, 1));
    hash.push_str(&base83_encode(dc_value, 4));

    for j in 0..components_y {
        for i in 0..components_x {
            if i == 0 && j == 0 {
                continue;
            }

            let factor = factors[j as usize][i as usize];

            let quant_r = (sign_pow(factor[0] / maximum_value, 0.5) * 9.0 + 9.5)
                .floor()
                .clamp(0.0, 18.0) as u32;
            let quant_g = (sign_pow(factor[1] / maximum_value, 0.5) * 9.0 + 9.5)
                .floor()
                .clamp(0.0, 18.0) as u32;
            let quant_b = (sign_pow(factor[2] / maximum_value, 0.5) * 9.0 + 9.5)
                .floor()
                .clamp(0.0, 18.0) as u32;

            let value = (quant_r * 19 * 19) + (quant_g * 19) + quant_b;

            hash.push_str(&base83_encode(value, 2));
        }
    }

    hash
}

fn decode_dc(value: u32) -> [f64; 3] {
    [
        srgb_to_linear(f64::from((value >> 16) & 0xFF) / 255.0),
        srgb_to_linear(f64::from((value >> 8) & 0xFF) / 255.0),
        srgb_to_linear(f64::from(value & 0xFF) / 255.0),
    ]
}

fn decode_ac(value: u32, maximum_value: f64) -> [f64; 3] {
    let quant_r = value / (19 * 19);
    let quant_g = (value / 19) % 19;
    let quant_b = value % 19;

    [
        sign_pow((f64::from(quant_r) - 9.0) / 9.0, 2.0) * maximum_value,
        sign_pow((f64::from(quant_g) - 9.0) / 9.0, 2.0) * maximum_value,
        sign_pow((f64::from(quant_b) - 9.0) / 9.0, 2.0) * maximum_value,
    ]
}

fn decode_blur_hash(blurhash: &str, width: u32, height: u32) -> HydrusResult<RgbImage> {
    if width == 0 || height == 0 {
        return Err(damaged("BlurHash decoding needs a non-zero resolution."));
    }

    if blurhash.len() < 6 {
        return Err(damaged("BlurHash is too short."));
    }

    let size_flag = base83_decode(&blurhash[0..1])?;
    let num_y = size_flag / 9;
    let num_x = (size_flag % 9) + 1;

    if num_y == 0 || num_y > 9 || num_x == 0 || num_x > 9 {
        return Err(damaged("BlurHash has an invalid component size."));
    }

    let expected_length = 4 + (2 * num_x * num_y) as usize;

    if blurhash.len() < expected_length {
        return Err(damaged(format!(
            "BlurHash needs {expected_length} characters but has {}.",
            blurhash.len()
        )));
    }

    let quantised_maximum_value = base83_decode(&blurhash[1..2])?;
    let maximum_value = f64::from(quantised_maximum_value + 1) / 166.0;

    let dc = decode_dc(base83_decode(&blurhash[2..6])?);

    let mut factors: Vec<Vec<[f64; 3]>> = vec![vec![[0.0; 3]; num_x as usize]; num_y as usize];

    let mut cursor = 6;

    for j in 0..num_y {
        for i in 0..num_x {
            if i == 0 && j == 0 {
                factors[0][0] = dc;
            } else {
                let value = base83_decode(&blurhash[cursor..cursor + 2])?;
                cursor += 2;

                factors[j as usize][i as usize] = decode_ac(value, maximum_value);
            }
        }
    }

    let mut pixels = Vec::with_capacity(width as usize * height as usize * 3);

    for y in 0..height {
        for x in 0..width {
            let mut colour = factors[0][0];

            for (j, row) in factors.iter().enumerate() {
                let y_basis = (PI * f64::from(y) * j as f64 / f64::from(height)).cos();

                for (i, factor) in row.iter().enumerate() {
                    if i == 0 && j == 0 {
                        continue;
                    }

                    let basis = (PI * f64::from(x) * i as f64 / f64::from(width)).cos() * y_basis;

                    for channel in 0..3 {
                        colour[channel] += factor[channel] * basis;
                    }
                }
            }

            for channel in colour {
                let encoded = linear_to_srgb(channel.clamp(0.0, 1.0));

                pixels.push((encoded * 255.0).round().clamp(0.0, 255.0) as u8);
            }
        }
    }

    RgbImage::from_raw(width, height, pixels)
        .ok_or_else(|| damaged("Could not construct the decoded BlurHash image."))
}

/// `get_numpy_from_blur_hash( blurhash, width, height )`
pub fn get_numpy_from_blur_hash(
    blurhash: &str,
    width: u32,
    height: u32,
) -> HydrusResult<DynamicImage> {
    if width == 0 || height == 0 {
        return Err(damaged("BlurHash decoding needs a non-zero resolution."));
    }

    if width > 32 || height > 32 {
        let small = decode_blur_hash(blurhash, 32, 32)?;

        Ok(hydrus_image_handling::resize_image(
            &DynamicImage::ImageRgb8(small),
            (width, height),
            None,
        ))
    } else {
        decode_blur_hash(blurhash, width, height).map(DynamicImage::ImageRgb8)
    }
}

#[cfg(test)]
mod tests {
    use image::{GenericImageView, Rgb, RgbImage};

    use super::*;

    fn solid_image(r: u8, g: u8, b: u8) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_pixel(32, 32, Rgb([r, g, b])))
    }

    #[test]
    fn base83_round_trips() {
        for value in [0_u32, 1, 82, 83, 12345, 83_u32.pow(4) - 1] {
            let encoded = base83_encode(value, 4);

            assert_eq!(base83_decode(&encoded).unwrap(), value);
        }
    }

    #[test]
    fn solid_colour_round_trips_through_encode_and_decode() {
        let image = solid_image(220, 40, 120);
        let hash = get_blur_hash_from_numpy(&image);

        assert_eq!(hash.len(), 36);
        assert_eq!(
            get_average_colour_from_blur_hash(&hash).unwrap(),
            (220, 40, 120)
        );

        let decoded = get_numpy_from_blur_hash(&hash, 8, 8).unwrap();

        assert_eq!(decoded.dimensions(), (8, 8));

        let [r, g, b, _] = decoded.to_rgba8().get_pixel(4, 4).0;

        assert!((i32::from(r) - 220).abs() <= 2);
        assert!((i32::from(g) - 40).abs() <= 2);
        assert!((i32::from(b) - 120).abs() <= 2);
    }

    #[test]
    fn average_colour_is_the_dc_component() {
        // The DC component of this hand-built hash is 0x123456.
        let hash = format!("{}{}", "00", base83_encode(0x123456, 4));

        assert_eq!(
            get_average_colour_from_blur_hash(&hash).unwrap(),
            (0x12, 0x34, 0x56)
        );
    }

    #[test]
    fn sort_helpers_return_finite_numbers() {
        let hash = get_blur_hash_from_numpy(&solid_image(20, 40, 200));

        let (cm, lightness) =
            convert_blur_hash_to_sortable_chromatic_magnitude(&hash, false).unwrap();
        let (blue_yellow, _) = convert_blur_hash_to_sortable_blue_yellow(&hash, false).unwrap();
        let (green_red, _) = convert_blur_hash_to_sortable_green_red(&hash, false).unwrap();
        let (initial, hue, saturation) = convert_blur_hash_to_sortable_hue(&hash, false).unwrap();

        assert!(cm.is_finite());
        assert!(lightness.is_finite());
        assert!(blue_yellow.is_finite());
        assert!(green_red.is_finite());
        assert_eq!(initial, 0.0);
        assert!((0.0..360.0).contains(&hue));
        assert!(saturation <= 0.0);
    }

    #[test]
    fn hsl_matches_known_primary_colours() {
        assert_eq!(rgb_to_hsl(255.0, 0.0, 0.0).0, 0.0);
        assert_eq!(rgb_to_hsl(0.0, 255.0, 0.0).0, 120.0);
        assert_eq!(rgb_to_hsl(0.0, 0.0, 255.0).0, 240.0);
        assert_eq!(rgb_to_hsl(128.0, 128.0, 128.0).1, 0.0);
    }

    #[test]
    fn oversized_hashes_decode_small_then_scale() {
        let hash = get_blur_hash_from_numpy(&solid_image(1, 2, 3));
        let decoded = get_numpy_from_blur_hash(&hash, 100, 50).unwrap();

        assert_eq!(decoded.dimensions(), (100, 50));
    }

    #[test]
    fn malformed_hashes_are_errors() {
        assert!(get_numpy_from_blur_hash("", 8, 8).is_err());
        assert!(get_numpy_from_blur_hash("~~~~~~", 8, 8).is_err());
        assert!(base83_decode("!").is_err());
    }
}
