//! Shape perceptual hashes from `ClientImagePerceptualHashes.py`.
//!
//! Python emulates OpenCV's DCT with an FFT on a mirrored 64x64 image. The
//! same maths is reproduced here with a direct DFT, which keeps the hash
//! bit-compatible without pulling in OpenCV.

use std::f64::consts::PI;

use image::DynamicImage;

pub const BLANK_PERCEPTUAL_HASH: [u8; 8] = [0; 8];

/// Composites over white and converts to OpenCV's RGB -> GRAY weights.
fn grayscale(image: &DynamicImage) -> (Vec<f64>, u32, u32) {
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut gray = Vec::with_capacity((width * height) as usize);

    for pixel in rgba.pixels() {
        let [r, g, b, a] = pixel.0;
        let bare = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
        let alpha = a as f64;

        gray.push(((bare * (alpha / 255.0)) + (255.0 - alpha)).floor());
    }

    (gray, width, height)
}

/// OpenCV `INTER_AREA`-style downscale, with nearest for upscales.
fn resize_area(gray: &[f64], width: u32, height: u32, target: u32) -> Vec<f64> {
    let mut resized = Vec::with_capacity((target * target) as usize);

    for y in 0..target {
        for x in 0..target {
            if width <= target || height <= target {
                let source_x = (x * width / target).min(width - 1);
                let source_y = (y * height / target).min(height - 1);
                resized.push(gray[(source_y * width + source_x) as usize]);
                continue;
            }

            let x0 = x * width / target;
            let x1 = (((x + 1) * width + target - 1) / target).min(width);
            let y0 = y * height / target;
            let y1 = (((y + 1) * height + target - 1) / target).min(height);
            let mut total = 0.0;
            let mut count = 0;

            for source_y in y0..y1 {
                for source_x in x0..x1 {
                    total += gray[(source_y * width + source_x) as usize];
                    count += 1;
                }
            }

            resized.push(total / count as f64);
        }
    }

    resized
}

fn norm_factor(k: usize) -> f64 {
    if k == 0 { 1.0 / 2.0_f64.sqrt() } else { 1.0 }
}

/// Port of Python's `PILDCT`.
fn pildct(gray: &[f64]) -> Vec<f64> {
    const SIDE: usize = 32;
    const EXTENDED: usize = 64;

    let mut extended = vec![0.0f64; EXTENDED * EXTENDED];

    for y in 0..SIDE {
        for x in 0..SIDE {
            let value = gray[y * SIDE + x];
            extended[y * EXTENDED + x] = value;
            extended[y * EXTENDED + (EXTENDED - 1 - x)] = value;
            extended[(EXTENDED - 1 - y) * EXTENDED + x] = value;
            extended[(EXTENDED - 1 - y) * EXTENDED + (EXTENDED - 1 - x)] = value;
        }
    }

    let mut result = vec![0.0f64; SIDE * SIDE];

    for u in 0..SIDE {
        let u_angle = -PI * u as f64 / EXTENDED as f64;

        for v in 0..SIDE {
            let v_angle = -PI * v as f64 / EXTENDED as f64;
            let (mut re, mut im) = (0.0, 0.0);

            for y in 0..EXTENDED {
                let y_angle = -2.0 * PI * (u * y) as f64 / EXTENDED as f64;
                let sin_y = y_angle.sin();
                let cos_y = y_angle.cos();

                for x in 0..EXTENDED {
                    let value = extended[y * EXTENDED + x];

                    if value == 0.0 {
                        continue;
                    }

                    let x_angle = -2.0 * PI * (v * x) as f64 / EXTENDED as f64;
                    let (sin_x, cos_x) = x_angle.sin_cos();

                    // exp(i*(y_angle + x_angle))
                    let cos_total = cos_y * cos_x - sin_y * sin_x;
                    let sin_total = sin_y * cos_x + cos_y * sin_x;

                    re += value * cos_total;
                    im += value * sin_total;
                }
            }

            // numpy's ortho norm for a 64x64 transform.
            re /= EXTENDED as f64;
            im /= EXTENDED as f64;

            // Multiply by Norm = exp(-i*u*pi/N) * exp(-i*v*pi/N) * scales.
            let scale = norm_factor(u) * norm_factor(v);
            let (sin_u, cos_u) = u_angle.sin_cos();
            let (sin_v, cos_v) = v_angle.sin_cos();
            let cos_norm = cos_u * cos_v - sin_u * sin_v;
            let sin_norm = sin_u * cos_v + cos_u * sin_v;

            result[u * SIDE + v] = (re * cos_norm - im * sin_norm) * scale;
        }
    }

    result
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

/// `GenerateShapePerceptualHashNumPy`
pub fn generate_shape_perceptual_hash(image: &DynamicImage) -> Vec<u8> {
    let (gray, width, height) = grayscale(image);
    let tiny = resize_area(&gray, width, height, 32);
    let dct = pildct(&tiny);

    let mut useful = Vec::with_capacity(64);

    for y in 0..8 {
        for x in 0..8 {
            useful.push(dct[y * 32 + x]);
        }
    }

    let mut rest = useful[1..].to_vec();
    let median = median(&mut rest);

    let mut hash = vec![0u8; 8];

    for (index, value) in useful.iter().enumerate() {
        if *value > median {
            hash[index / 8] |= 1 << (7 - (index % 8));
        }
    }

    hash
}

/// `DiscardBlankPerceptualHashes`
pub fn discard_blank_perceptual_hashes(hashes: impl IntoIterator<Item = Vec<u8>>) -> Vec<Vec<u8>> {
    hashes
        .into_iter()
        .filter(|hash| hamming_distance(hash, &BLANK_PERCEPTUAL_HASH) > 4)
        .collect()
}

fn hamming_distance(left: &[u8], right: &[u8]) -> u32 {
    left.iter()
        .zip(right)
        .map(|(a, b)| (a ^ b).count_ones())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn hash_is_eight_bytes_and_stable() {
        let image =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(64, 64, Rgba([10, 20, 30, 255])));

        let first = generate_shape_perceptual_hash(&image);
        let second = generate_shape_perceptual_hash(&image);

        assert_eq!(first.len(), 8);
        assert_eq!(first, second);
    }

    #[test]
    fn blank_hashes_are_discarded() {
        let hashes =
            discard_blank_perceptual_hashes(vec![BLANK_PERCEPTUAL_HASH.to_vec(), vec![0xff; 8]]);

        assert_eq!(hashes, vec![vec![0xff; 8]]);
    }

    #[test]
    fn matches_python_hydrus_fixture_when_available() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../hydrus_python/static/hydrus.png");

        if !path.exists() {
            return;
        }

        let image = image::open(&path).unwrap();
        let hash = generate_shape_perceptual_hash(&image);

        assert_eq!(hash, vec![0xb4, 0x4d, 0xc7, 0xb2, 0x4d, 0xcb, 0x38, 0x1c]);
    }
}
