use std::f64::consts::PI;

use hydrus_core::hydrus_exceptions::HydrusResult;

use super::colour::{linear_to_srgb, sign_pow, srgb_to_linear};
use super::value_error;

use hydrus_core::files::images::hydrus_blurhash::base83_decode;

/// `blurhash_components( blurhash )`
pub fn blurhash_components(blurhash: &str) -> HydrusResult<(u32, u32)> {
    if blurhash.len() < 6 {
        return Err(value_error("Blurhash must be at least 6 characters long."));
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
        return Err(value_error(
            "Blurhash decoding needs a non-zero resolution.",
        ));
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
            sign_pow((f64::from(ac_value / (19 * 19)) - 9.0) / 9.0, 2.0) * real_max_value,
            sign_pow((f64::from(ac_value / 19 % 19) - 9.0) / 9.0, 2.0) * real_max_value,
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
