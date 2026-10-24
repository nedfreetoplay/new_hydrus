//! Client image helpers from `hydrus/client/files/images/ClientImageHandling.py`
//! and friends. Most work delegates to the already ported `hydrus_core`.

pub mod metadata;
pub mod perceptual;

use std::path::Path;

use hydrus_core::files::images::hydrus_image_handling as core_image;
use hydrus_core::hydrus_constants::Mime;
use hydrus_core::hydrus_exceptions::HydrusResult;
use image::DynamicImage;

/// `ResizeNumPyImageForMediaViewer`
pub fn resize_numpy_image_for_media_viewer(
    _mime: Mime,
    image: &DynamicImage,
    target_resolution: (u32, u32),
) -> DynamicImage {
    core_image::resize_image(image, target_resolution, None)
}

pub fn get_image_resolution(path: impl AsRef<Path>, mime: Mime) -> HydrusResult<(u32, u32)> {
    core_image::get_image_resolution(path, mime)
}

pub fn decode_image_path(path: impl AsRef<Path>) -> HydrusResult<DynamicImage> {
    core_image::decode_image_path(path)
}

pub fn generate_thumbnail_bytes(image: &DynamicImage) -> HydrusResult<Vec<u8>> {
    core_image::generate_thumbnail_bytes_from_numpy(image)
}

pub fn has_transparency(image: &DynamicImage) -> bool {
    hydrus_core::files::images::hydrus_image_colours::image_has_human_perceptible_alpha_channel(
        image,
    )
}

pub fn get_blur_hash(image: &DynamicImage) -> String {
    hydrus_core::files::images::hydrus_blurhash::get_blur_hash_from_numpy(image)
}

pub use metadata::{ImageMetadataFlags, get_metadata_flags, read_image_metadata};
pub use perceptual::{
    BLANK_PERCEPTUAL_HASH, discard_blank_perceptual_hashes, generate_shape_perceptual_hash,
};

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn image(width: u32, height: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            width,
            height,
            Rgba([10, 20, 30, 255]),
        ))
    }

    #[test]
    fn resize_matches_target_resolution() {
        let resized = resize_numpy_image_for_media_viewer(Mime::Png, &image(100, 50), (20, 20));

        assert_eq!(resized.width(), 20);
        assert_eq!(resized.height(), 20);
    }

    #[test]
    fn thumbnail_bytes_are_encoded() {
        let bytes = generate_thumbnail_bytes(&image(16, 16)).unwrap();

        assert!(!bytes.is_empty());
    }

    #[test]
    fn perceptual_hash_is_stable() {
        let first = generate_shape_perceptual_hash(&image(32, 32));
        let second = generate_shape_perceptual_hash(&image(32, 32));

        assert_eq!(first, second);
        assert_eq!(first.len(), 8);
    }

    #[test]
    fn opaque_images_have_no_transparency() {
        assert!(!has_transparency(&image(4, 4)));
    }
}
