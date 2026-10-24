
use super::*;
use image::{Rgba, RgbaImage};

#[test]
fn hash_is_eight_bytes_and_stable() {
    let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(64, 64, Rgba([10, 20, 30, 255])));

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
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../hydrus_python/static/hydrus.png");

    if !path.exists() {
        return;
    }

    let image = image::open(&path).unwrap();
    let hash = generate_shape_perceptual_hash(&image);

    assert_eq!(hash, vec![0xb4, 0x4d, 0xc7, 0xb2, 0x4d, 0xcb, 0x38, 0x1c]);
}
