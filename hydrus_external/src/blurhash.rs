//! Rust port of `hydrus_python/hydrus/external/blurhash.py`.
//!
//! Pixel matrices use `[f64; 3]` rows, matching the Python module's nested
//! lists. Encoding accepts an [`image::DynamicImage`] for convenience.

use std::f64::consts::PI;

use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

pub use hydrus_core::files::images::hydrus_blurhash::{
    base83_decode, base83_encode, get_average_colour_from_blur_hash,
    get_blur_hash_from_numpy, get_numpy_from_blur_hash,
};

fn value_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::Parse, message)
}

/// `srgb_to_linear( value )`
pub fn srgb_to_linear(value: f64) -> f64 {
    let value = value / 255.0;

    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// `sign_pow( value, exp )`
pub fn sign_pow(value: f64, exponent: f64) -> f64 {
    value.abs().powf(exponent).copysign(value)
}

/// `linear_to_srgb( value )`
pub fn linear_to_srgb(value: f64) -> u8 {
    let value = value.clamp(0.0, 1.0);

    let converted = if value <= 0.0031308 {
        value * 12.92
    } else {
        (1.055 * value.powf(1.0 / 2.4)) - 0.055
    };

    (converted * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}

/// `blurhash_components( blurhash )`
pub fn blurhash_components(blurhash: &str) -> HydrusResult<(u32, u32)> {
    if blurhash.len() < 6 {
        return Err(value_error(
            "Blurhash must be at least 6 characters long.",
        ));
    }

    let size_info = base83_decode(&blurhash[0..1])?;
    let size_y = size_info / 9 + 1;
    let size_x = (size_info % 9) + 1;

    Ok((size_x, size_y))
}

/// `blurhash_decode( blurhash, width, height, punch = 1.0, linear = False )`
pub fn blurhash_decode(
    blurhash: &str,
    width: usize,
    height: usize,
    punch: f64,
    linear: bool,
) -> HydrusResult<Vec<Vec<[f64; 3]>>> {
    let (size_x, size_y) = blurhash_components(blurhash)?;

    if blurhash.len() != (4 + 2 * size_x as usize * size_y as usize) {
        return Err(value_error("Invalid Blurhash length."));
    }

    if width == 0 || height == 0 {
        return Err(value_error("Blurhash decoding needs a non-zero resolution."));
    }

    let quant_max_value = base83_decode(&blurhash[1..2])?;
    let real_max_value = (f64::from(quant_max_value) + 1.0) / 166.0 * punch;

    let dc_value = base83_decode(&blurhash[2..6])?;

    let mut colours = vec![[
        srgb_to_linear(f64::from((dc_value >> 16) & 0xFF)),
        srgb_to_linear(f64::from((dc_value >> 8) & 0xFF)),
        srgb_to_linear(f64::from(dc_value & 0xFF)),
    ]];

    for component in 1..(size_x * size_y) as usize {
        let start = 4 + component * 2;
        let ac_value = base83_decode(&blurhash[start..start + 2])?;

        colours.push([
            sign_pow((f64::from(ac_value / (19 * 19)) - 9.0) / 9.0, 2.0)
                * real_max_value,
            sign_pow((f64::from(ac_value / 19 % 19) - 9.0) / 9.0, 2.0)
                * real_max_value,
            sign_pow((f64::from(ac_value % 19) - 9.0) / 9.0, 2.0) * real_max_value,
        ]);
    }

    let mut pixels = Vec::with_capacity(height);

    for y in 0..height {
        let mut row = Vec::with_capacity(width);

        for x in 0..width {
            let mut pixel = [0.0_f64; 3];

            for j in 0..size_y as usize {
                for i in 0..size_x as usize {
                    let basis = (PI * x as f64 * i as f64 / width as f64).cos()
                        * (PI * y as f64 * j as f64 / height as f64).cos();
                    let colour = colours[i + j * size_x as usize];

                    pixel[0] += colour[0] * basis;
                    pixel[1] += colour[1] * basis;
                    pixel[2] += colour[2] * basis;
                }
            }

            if linear {
                row.push(pixel);
            } else {
                row.push([
                    f64::from(linear_to_srgb(pixel[0])),
                    f64::from(linear_to_srgb(pixel[1])),
                    f64::from(linear_to_srgb(pixel[2])),
                ]);
            }
        }

        pixels.push(row);
    }

    Ok(pixels)
}

/// `blurhash_encode( image, components_x = 4, components_y = 4, linear = False )`
pub fn blurhash_encode(
    image: &image::DynamicImage,
    components_x: u32,
    components_y: u32,
    linear: bool,
) -> HydrusResult<String> {
    if !(1..=9).contains(&components_x) || !(1..=9).contains(&components_y) {
        return Err(value_error(
            "x and y component counts must be between 1 and 9 inclusive.",
        ));
    }

    let rgb = image.to_rgb8();
    let width = rgb.width().max(1) as usize;
    let height = rgb.height().max(1) as usize;

    let mut components = Vec::with_capacity((components_x * components_y) as usize);
    let mut max_ac_component = 0.0_f64;

    for j in 0..components_y {
        for i in 0..components_x {
            let norm_factor = if i == 0 && j == 0 { 1.0 } else { 2.0 };
            let mut component = [0.0_f64; 3];

            for y in 0..height {
                for x in 0..width {
                    let basis = norm_factor
                        * (PI * f64::from(i) * x as f64 / width as f64).cos()
                        * (PI * f64::from(j) * y as f64 / height as f64).cos();
                    let pixel = rgb.get_pixel(x as u32, y as u32).0;

                    for (channel, value) in pixel.iter().enumerate() {
                        let value = if linear {
                            f64::from(*value)
                        } else {
                            srgb_to_linear(f64::from(*value))
                        };

                        component[channel] += basis * value;
                    }
                }
            }

            for channel in &mut component {
                *channel /= (width * height) as f64;
            }

            components.push(component);

            if !(i == 0 && j == 0) {
                max_ac_component = max_ac_component
                    .max(component[0].abs())
                    .max(component[1].abs())
                    .max(component[2].abs());
            }
        }
    }

    let dc_value = (u32::from(linear_to_srgb(components[0][0])) << 16)
        + (u32::from(linear_to_srgb(components[0][1])) << 8)
        + u32::from(linear_to_srgb(components[0][2]));

    let quant_max_ac_component =
        ((max_ac_component * 166.0 - 0.5).floor()).clamp(0.0, 82.0) as u32;
    let norm_factor = (f64::from(quant_max_ac_component) + 1.0) / 166.0;

    let mut blurhash = String::new();
    blurhash.push_str(&base83_encode((components_x - 1) + (components_y - 1) * 9, 1));
    blurhash.push_str(&base83_encode(quant_max_ac_component, 1));
    blurhash.push_str(&base83_encode(dc_value, 4));

    for component in &components[1..] {
        let encode_channel = |value: f64| -> u32 {
            (sign_pow(value / norm_factor, 0.5) * 9.0 + 9.5)
                .floor()
                .clamp(0.0, 18.0) as u32
        };

        let ac_value = encode_channel(component[0]) * 19 * 19
            + encode_channel(component[1]) * 19
            + encode_channel(component[2]);

        blurhash.push_str(&base83_encode(ac_value, 2));
    }

    Ok(blurhash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base83_and_components_round_trip() {
        let encoded = base83_encode(12345, 4);

        assert_eq!(base83_decode(&encoded).unwrap(), 12345);
        assert_eq!(blurhash_components("00abcde").unwrap(), (1, 1));
    }

    #[test]
    fn solid_colour_encode_decode_round_trips() {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            8,
            8,
            image::Rgb([220, 40, 120]),
        ));
        let hash = blurhash_encode(&image, 4, 4, false).unwrap();
        let pixels = blurhash_decode(&hash, 8, 8, 1.0, false).unwrap();

        assert_eq!(pixels.len(), 8);
        assert_eq!(pixels[0].len(), 8);
        assert!((pixels[4][4][0] - 220.0).abs() <= 3.0);
        assert!((pixels[4][4][1] - 40.0).abs() <= 3.0);
        assert!((pixels[4][4][2] - 120.0).abs() <= 3.0);
    }

    #[test]
    fn linear_decode_returns_floats() {
        let hash = blurhash_encode(
            &image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                4,
                4,
                image::Rgb([10, 20, 30]),
            )),
            2,
            2,
            false,
        )
        .unwrap();

        let pixels = blurhash_decode(&hash, 2, 2, 1.0, true).unwrap();

        assert!(pixels[0][0][0] <= 1.0);
    }
}
