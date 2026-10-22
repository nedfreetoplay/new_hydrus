//! Rust port of `hydrus/core/files/images/HydrusImageColours.py`.
//!
//! Python works with numpy arrays; the Rust port works with decoded
//! [`DynamicImage`] values. The useful-alpha rules are kept exactly, including
//! the "human perceptible" thresholds that stop anti-aliased borders from
//! forcing every thumbnail to be a PNG.

use std::sync::atomic::{AtomicU8, Ordering};

use image::DynamicImage;

/// `HAS_TRANSPARENCY_STRICTNESS_CHANNEL_PRESENCE`
pub const HAS_TRANSPARENCY_STRICTNESS_CHANNEL_PRESENCE: u8 = 0;
/// `HAS_TRANSPARENCY_STRICTNESS_NOT_BLACK_OR_WHITE`
pub const HAS_TRANSPARENCY_STRICTNESS_NOT_BLACK_OR_WHITE: u8 = 1;
/// `HAS_TRANSPARENCY_STRICTNESS_HUMAN`
pub const HAS_TRANSPARENCY_STRICTNESS_HUMAN: u8 = 2;

pub const HAS_TRANSPARENCY_STRICTNESS_STRING_LOOKUP: &[(u8, &str)] = &[
    (
        HAS_TRANSPARENCY_STRICTNESS_CHANNEL_PRESENCE,
        "it has a transparency channel",
    ),
    (
        HAS_TRANSPARENCY_STRICTNESS_NOT_BLACK_OR_WHITE,
        "it has a transparency channel that is not completely transparent or opaque",
    ),
    (
        HAS_TRANSPARENCY_STRICTNESS_HUMAN,
        "it has a transparency channel that a human might recognise",
    ),
];

static CURRENT_HAS_TRANSPARENCY_STRICTNESS_LEVEL: AtomicU8 =
    AtomicU8::new(HAS_TRANSPARENCY_STRICTNESS_HUMAN);

#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// `set_has_transparency_strictness_level( value )`
///
/// Python clears the image caches here through `HG.controller.pub`; the Rust
/// core has no controller dependency, so callers that cache thumbnails should
/// clear their own caches after calling this.
pub fn set_has_transparency_strictness_level(value: u8) {
    CURRENT_HAS_TRANSPARENCY_STRICTNESS_LEVEL.store(value, Ordering::Relaxed);
}

pub fn current_has_transparency_strictness_level() -> u8 {
    CURRENT_HAS_TRANSPARENCY_STRICTNESS_LEVEL.load(Ordering::Relaxed)
}

pub fn has_transparency_strictness_string_lookup(level: u8) -> Option<&'static str> {
    HAS_TRANSPARENCY_STRICTNESS_STRING_LOOKUP
        .iter()
        .find(|(value, _)| *value == level)
        .map(|(_, text)| *text)
}

/// `numpy_image_has_alpha_channel( numpy_image )`
pub fn image_has_alpha_channel(image: &DynamicImage) -> bool {
    image.color().has_alpha()
}

/// `get_numpy_alpha_channel( numpy_image )`
pub fn get_image_alpha_channel(image: &DynamicImage) -> Option<Vec<u8>> {
    if !image_has_alpha_channel(image) {
        return None;
    }

    Some(
        image
            .to_rgba8()
            .pixels()
            .map(|pixel| pixel[3])
            .collect(),
    )
}

fn alpha_is_all(alpha: &[u8], value: u8) -> bool {
    alpha.iter().all(|alpha_value| *alpha_value == value)
}

/// `numpy_image_has_non_fully_black_or_white_alpha_channel( numpy_image )`
pub fn image_has_non_fully_black_or_white_alpha_channel(image: &DynamicImage) -> bool {
    let Some(alpha) = get_image_alpha_channel(image) else {
        return false;
    };

    if alpha_is_all(&alpha, 0) || alpha_is_all(&alpha, 255) {
        return false;
    }

    true
}

/// `numpy_image_has_human_perceptible_alpha_channel( numpy_image )`
pub fn image_has_human_perceptible_alpha_channel(image: &DynamicImage) -> bool {
    let Some(alpha) = get_image_alpha_channel(image) else {
        return false;
    };

    let width = image.width() as usize;
    let height = image.height() as usize;

    let circumference = 2 * (width + height);
    let weight = std::cmp::max((width * height) / 200, 1);
    let num_interesting_alpha_pixels_needed = std::cmp::min(circumference, weight);

    // Do not combine these tests: one says "all opaque", the other "all
    // transparent", and both matter.
    let clustered_in_opaque_land = alpha
        .iter()
        .filter(|alpha_value| **alpha_value < 251)
        .count();

    if clustered_in_opaque_land < num_interesting_alpha_pixels_needed {
        return false;
    }

    let clustered_in_transparency_land = alpha
        .iter()
        .filter(|alpha_value| **alpha_value > 4)
        .count();

    if clustered_in_transparency_land < num_interesting_alpha_pixels_needed {
        return false;
    }

    true
}

/// `numpy_image_has_useful_alpha_channel( numpy_image )`
pub fn image_has_useful_alpha_channel(image: &DynamicImage) -> bool {
    match current_has_transparency_strictness_level() {
        HAS_TRANSPARENCY_STRICTNESS_HUMAN => {
            image_has_human_perceptible_alpha_channel(image)
        }
        HAS_TRANSPARENCY_STRICTNESS_NOT_BLACK_OR_WHITE => {
            image_has_non_fully_black_or_white_alpha_channel(image)
        }
        HAS_TRANSPARENCY_STRICTNESS_CHANNEL_PRESENCE => image_has_alpha_channel(image),
        _ => false,
    }
}

/// `numpy_image_has_useless_alpha_channel( numpy_image )`
pub fn image_has_useless_alpha_channel(image: &DynamicImage) -> bool {
    image_has_alpha_channel(image) && !image_has_useful_alpha_channel(image)
}

/// `pil_image_has_transparency( pil_image )`
///
/// A decoded palette image with a `transparency` entry is already RGBA in the
/// `image` crate, so `has_alpha` is the right equivalent.
pub fn pil_image_has_transparency(image: &DynamicImage) -> bool {
    image_has_alpha_channel(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn rgba_image(pixels: &[[u8; 4]], width: u32, height: u32) -> DynamicImage {
        let mut buffer = RgbaImage::new(width, height);

        for index in 0..(width * height) as usize {
            let pixel = pixels[index % pixels.len()];
            let x = index as u32 % width;
            let y = index as u32 / width;
            buffer.put_pixel(x, y, Rgba(pixel));
        }

        DynamicImage::ImageRgba8(buffer)
    }

    #[test]
    fn an_rgb_image_has_no_alpha_channel() {
        let image = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            10,
            10,
            image::Rgb([1, 2, 3]),
        ));

        assert!(!image_has_alpha_channel(&image));
        assert!(!image_has_useless_alpha_channel(&image));
        assert!(!pil_image_has_transparency(&image));
    }

    #[test]
    fn a_fully_opaque_alpha_channel_is_useless() {
        let _guard = TEST_LOCK.lock().unwrap();

        let image = rgba_image(&[[1, 2, 3, 255]], 10, 10);

        assert!(image_has_alpha_channel(&image));
        assert!(!image_has_non_fully_black_or_white_alpha_channel(&image));
        assert!(!image_has_human_perceptible_alpha_channel(&image));
        assert!(image_has_useless_alpha_channel(&image));
    }

    #[test]
    fn a_fully_transparent_alpha_channel_is_useless() {
        let _guard = TEST_LOCK.lock().unwrap();

        let image = rgba_image(&[[1, 2, 3, 0]], 10, 10);

        assert!(!image_has_non_fully_black_or_white_alpha_channel(&image));
        assert!(!image_has_human_perceptible_alpha_channel(&image));
        assert!(image_has_useless_alpha_channel(&image));
    }

    #[test]
    fn a_single_interesting_alpha_pixel_is_enough_for_a_small_image() {
        let _guard = TEST_LOCK.lock().unwrap();

        let mut pixels = vec![[1, 2, 3, 255]; 100];
        pixels[0][3] = 128;

        let image = rgba_image(&pixels, 10, 10);

        assert!(image_has_non_fully_black_or_white_alpha_channel(&image));
        assert!(image_has_human_perceptible_alpha_channel(&image));
        assert!(image_has_useful_alpha_channel(&image));
        assert!(!image_has_useless_alpha_channel(&image));
    }

    #[test]
    fn the_strictness_level_changes_the_answer() {
        let _guard = TEST_LOCK.lock().unwrap();

        let image = rgba_image(&[[1, 2, 3, 255]], 10, 10);

        set_has_transparency_strictness_level(HAS_TRANSPARENCY_STRICTNESS_CHANNEL_PRESENCE);
        assert!(image_has_useful_alpha_channel(&image));

        set_has_transparency_strictness_level(HAS_TRANSPARENCY_STRICTNESS_HUMAN);
        assert!(!image_has_useful_alpha_channel(&image));
    }

    #[test]
    fn string_lookup_is_available() {
        assert_eq!(
            has_transparency_strictness_string_lookup(HAS_TRANSPARENCY_STRICTNESS_HUMAN),
            Some("it has a transparency channel that a human might recognise")
        );
    }
}
