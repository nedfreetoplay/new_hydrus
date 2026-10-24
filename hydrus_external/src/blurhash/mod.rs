//! Rust port of `hydrus_python/hydrus/external/blurhash.py`.
//!
//! Pixel matrices use `[f64; 3]` rows, matching the Python module's nested
//! lists. Encoding accepts an [`image::DynamicImage`] for convenience.

mod colour;
mod decode;
mod encode;

pub use decode::{blurhash_components, blurhash_decode};
pub use encode::blurhash_encode;

pub use colour::{linear_to_srgb, sign_pow, srgb_to_linear};
pub(crate) fn value_error(message: impl Into<String>) -> hydrus_core::hydrus_exceptions::HydrusException {
    hydrus_core::hydrus_exceptions::HydrusException::new(
        hydrus_core::hydrus_exceptions::ErrorKind::Parse,
        message,
    )
}

pub use hydrus_core::files::images::hydrus_blurhash::{
    base83_decode, base83_encode, get_average_colour_from_blur_hash, get_blur_hash_from_numpy,
    get_numpy_from_blur_hash,
};

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
