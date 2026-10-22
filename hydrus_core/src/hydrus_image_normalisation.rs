//! Rust port of the useful parts of
//! `hydrus/core/files/images/HydrusImageNormalisation.py`.
//!
//! The byte-depth normalisation, alpha removal, EXIF rotation, and PNG
//! gamma/chromaticity conversion are ported. Arbitrary ICC profile transforms
//! still need a colour-management engine, so that one branch keeps Python's
//! safe no-op behaviour.

use std::sync::atomic::{AtomicBool, Ordering};

use image::metadata::Orientation;
use image::{DynamicImage, Rgba, RgbaImage};

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_image_colours;
use crate::hydrus_image_icc_profiles;
use crate::hydrus_image_math_slop::{self, Matrix3};
use crate::hydrus_image_metadata::{self, MetadataMap, MetadataValue};

static DO_ICC_PROFILE_NORMALISATION: AtomicBool = AtomicBool::new(true);

/// `set_do_icc_profile_normalisation( value )`
///
/// The flag is retained for callers that want to mirror the Python setting.
/// Actual ICC conversion is not implemented in the Rust core yet.
pub fn set_do_icc_profile_normalisation(value: bool) {
    DO_ICC_PROFILE_NORMALISATION.store(value, Ordering::Relaxed);
}

pub fn do_icc_profile_normalisation() -> bool {
    DO_ICC_PROFILE_NORMALISATION.load(Ordering::Relaxed)
}

fn is_grayscale(image: &DynamicImage) -> bool {
    matches!(
        image,
        DynamicImage::ImageLuma8(_)
            | DynamicImage::ImageLuma16(_)
            | DynamicImage::ImageLumaA8(_)
            | DynamicImage::ImageLumaA16(_)
    )
}

/// `normalise_numpy_image_to_uint8( numpy_image )`
///
/// `image` represents 16-bit and floating-point images directly; convert them
/// to their 8-bit equivalents. The float conversion uses the sample conversion
/// supplied by the `image` crate (a min/max stretch is not meaningful for the
/// already-normalised `f32` dynamic-image variants).
pub fn normalise_image_to_uint8(image: &DynamicImage) -> DynamicImage {
    match image {
        DynamicImage::ImageLuma16(_) => DynamicImage::ImageLuma8(image.to_luma8()),
        DynamicImage::ImageLumaA16(_) => {
            DynamicImage::ImageLumaA8(image.to_luma_alpha8())
        }
        DynamicImage::ImageRgb16(_) => DynamicImage::ImageRgb8(image.to_rgb8()),
        DynamicImage::ImageRgba16(_) => DynamicImage::ImageRgba8(image.to_rgba8()),
        DynamicImage::ImageRgb32F(_) => DynamicImage::ImageRgb8(image.to_rgb8()),
        DynamicImage::ImageRgba32F(_) => DynamicImage::ImageRgba8(image.to_rgba8()),
        _ => image.clone(),
    }
}

/// `dequantize_numpy_image( numpy_image )` / the numeric part of
/// `dequantize_pil_image`.
pub fn dequantize_image(image: &DynamicImage) -> DynamicImage {
    normalise_image_to_uint8(image)
}

/// `strip_out_any_alpha_channel( numpy_image )`
pub fn strip_out_any_alpha_channel(image: &DynamicImage) -> DynamicImage {
    if !hydrus_image_colours::image_has_alpha_channel(image) {
        return image.clone();
    }

    if is_grayscale(image) {
        DynamicImage::ImageLuma8(image.to_luma8())
    } else {
        DynamicImage::ImageRgb8(image.to_rgb8())
    }
}

/// `strip_out_any_useless_alpha_channel( numpy_image )`
pub fn strip_out_any_useless_alpha_channel(image: &DynamicImage) -> DynamicImage {
    if hydrus_image_colours::image_has_useless_alpha_channel(image) {
        strip_out_any_alpha_channel(image)
    } else {
        image.clone()
    }
}

/// The EXIF orientation is applied by the decoder in `hydrus_image_handling`;
/// this helper exists for callers that already have an orientation value.
pub fn apply_orientation(image: &mut DynamicImage, orientation: image::metadata::Orientation) {
    image.apply_orientation(orientation);
}

const FALLBACK_PNG_GAMMA: f64 = 0.45455;

const XYZ_TO_SRGB: Matrix3 = [
    [3.2404542, -1.5371385, -0.4985314],
    [-0.9692660, 1.8760108, 0.0415560],
    [0.0556434, -0.2040259, 1.0572252],
];

fn numeric_metadata_value(value: &MetadataValue) -> Option<f64> {
    match value {
        MetadataValue::Int(value) => Some(*value as f64),
        MetadataValue::Float(value) => Some(*value),
        _ => None,
    }
}

/// `get_linear_gamma_from_pil_png( pil_image )`, using the already-collected
/// PNG `info` dictionary.
pub fn get_linear_gamma_from_pil_png(info: &MetadataMap) -> f64 {
    info.get("gamma")
        .and_then(numeric_metadata_value)
        .unwrap_or(FALLBACK_PNG_GAMMA)
}

/// Read the eight `cHRM` values as white/red/green/blue `xy` pairs.
pub fn png_chromaticities_from_info(info: &MetadataMap) -> Option<[(f64, f64); 4]> {
    let MetadataValue::List(values) = info.get("chromaticity")? else {
        return None;
    };

    if values.len() != 8 {
        return None;
    }

    let mut numbers = [0.0_f64; 8];

    for (index, value) in values.iter().enumerate() {
        numbers[index] = numeric_metadata_value(value)?;
    }

    Some([
        (numbers[0], numbers[1]),
        (numbers[2], numbers[3]),
        (numbers[4], numbers[5]),
        (numbers[6], numbers[7]),
    ])
}

/// `pil_image_is_png_with_gamma_and_chromaticity( pil_image )`.
pub fn pil_image_is_png_with_gamma_and_chromaticity(
    format: &str,
    mode: &str,
    info: &MetadataMap,
) -> bool {
    if format != "PNG" || !matches!(mode, "RGB" | "RGBA") {
        return false;
    }

    if let Some(gamma) = info.get("gamma").and_then(numeric_metadata_value) {
        if gamma == 0.0 {
            return false;
        }
    }

    png_chromaticities_from_info(info)
        .is_some_and(|chromaticities| chromaticities.iter().all(|(x, y)| *x != 0.0 && *y != 0.0))
}

/// `pil_image_is_png_with_srgb( pil_image )`.
pub fn pil_image_is_png_with_srgb(format: &str, info: &MetadataMap) -> bool {
    format == "PNG" && info.contains_key("srgb")
}

/// `generate_icc_profile_bytes_from_gamma_and_chromaticity_png( pil_image )`.
pub fn generate_icc_profile_bytes_from_gamma_and_chromaticity_png(
    info: &MetadataMap,
) -> HydrusResult<Vec<u8>> {
    let linear_gamma = get_linear_gamma_from_pil_png(info);

    if linear_gamma == 0.0 {
        return Err(HydrusException::new(
            ErrorKind::DataMissing,
            "The PNG gamma is zero, so it cannot be converted to an ICC profile.",
        ));
    }

    let chromaticities = png_chromaticities_from_info(info).ok_or_else(|| {
        HydrusException::new(
            ErrorKind::DataMissing,
            "This PNG has no usable chromaticity information.",
        )
    })?;

    let rgb_to_xyz = hydrus_image_math_slop::chromaticities_to_rgb_to_xyz(
        chromaticities[0],
        chromaticities[1],
        chromaticities[2],
        chromaticities[3],
    );
    let rgb_to_xyz_d50 = hydrus_image_math_slop::adapt_rgb_to_xyz_matrix_to_d50(
        rgb_to_xyz,
        chromaticities[0],
    );

    Ok(hydrus_image_icc_profiles::make_gamma_and_chromaticity_icc_profile(
        1.0 / linear_gamma,
        rgb_to_xyz_d50,
    ))
}

/// `convert_gamma_chromaticity_png_to_srgb( pil_image )`.
///
/// The Python source implements this with numpy tiles. The arithmetic is the
/// same here; small images are cheap enough that the extra tiling bookkeeping is
/// not needed.
pub fn convert_gamma_chromaticity_png_to_srgb(
    image: &DynamicImage,
    info: &MetadataMap,
) -> HydrusResult<DynamicImage> {
    let Some(chromaticities) = png_chromaticities_from_info(info) else {
        return Ok(image.clone());
    };

    let linear_gamma = get_linear_gamma_from_pil_png(info);

    if linear_gamma == 0.0 {
        return Ok(image.clone());
    }

    let display_gamma = 1.0 / linear_gamma;
    let rgb_to_xyz = hydrus_image_math_slop::chromaticities_to_rgb_to_xyz(
        chromaticities[0],
        chromaticities[1],
        chromaticities[2],
        chromaticities[3],
    );
    let total_transform = hydrus_image_math_slop::mat_mul(XYZ_TO_SRGB, rgb_to_xyz);

    let has_alpha = image.color().has_alpha();
    let source = image.to_rgba8();
    let mut converted = RgbaImage::new(source.width(), source.height());

    for (x, y, pixel) in source.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;

        let linear = [
            (f64::from(r) / 255.0).powf(display_gamma),
            (f64::from(g) / 255.0).powf(display_gamma),
            (f64::from(b) / 255.0).powf(display_gamma),
        ];

        let transformed =
            hydrus_image_math_slop::mat_vec_mul(total_transform, linear);

        let encoded = transformed.map(|value| {
            let value = value.clamp(0.0, 1.0);

            (hydrus_image_math_slop::srgb_encode(value) * 255.0)
                .round_ties_even()
                .clamp(0.0, 255.0) as u8
        });

        converted.put_pixel(x, y, Rgba([encoded[0], encoded[1], encoded[2], a]));
    }

    if has_alpha {
        Ok(DynamicImage::ImageRgba8(converted))
    } else {
        Ok(DynamicImage::ImageRgb8(
            DynamicImage::ImageRgba8(converted).to_rgb8(),
        ))
    }
}

/// `normalise_icc_profile_pil_image_to_srgb( icc_profile_bytes, pil_image )`.
///
/// Parsing and applying arbitrary ICC transforms needs a colour-management
/// engine. The core port keeps the API and leaves the image unchanged, matching
/// Python's error/no-op path for profiles libjpeg/PIL cannot build a transform
/// for.
pub fn normalise_icc_profile_pil_image_to_srgb(
    _icc_profile_bytes: &[u8],
    image: &DynamicImage,
) -> DynamicImage {
    image.clone()
}

/// `normalise_pil_image_to_rgb( pil_image )`.
pub fn normalise_pil_image_to_rgb(image: &DynamicImage) -> DynamicImage {
    if image.color().has_alpha() {
        DynamicImage::ImageRgba8(image.to_rgba8())
    } else {
        DynamicImage::ImageRgb8(image.to_rgb8())
    }
}

/// `rotate_exif_pil_image( pil_image )`, with the orientation supplied by the
/// metadata reader. PNG is skipped as in the Python source.
pub fn rotate_exif_pil_image(
    image: &DynamicImage,
    format: &str,
    exif_orientation: Option<u8>,
) -> DynamicImage {
    if format == "PNG" {
        return image.clone();
    }

    let Some(orientation) = exif_orientation.and_then(Orientation::from_exif) else {
        return image.clone();
    };

    let mut rotated = image.clone();
    rotated.apply_orientation(orientation);

    rotated
}

/// `dequantize_pil_image( pil_image )`.
///
/// ICC transforms remain a no-op (see
/// [`normalise_icc_profile_pil_image_to_srgb`]); the sRGB and
/// gamma/chromaticity paths are applied directly.
pub fn dequantize_pil_image(
    image: &DynamicImage,
    format: &str,
    mode: &str,
    info: &MetadataMap,
) -> DynamicImage {
    let mut dequantised = image.clone();

    if hydrus_image_metadata::has_icc_profile(info) && do_icc_profile_normalisation() {
        dequantised = normalise_icc_profile_pil_image_to_srgb(
            &hydrus_image_metadata::get_icc_profile_bytes(info)
                .unwrap_or_default(),
            &dequantised,
        );
    } else if pil_image_is_png_with_srgb(format, info) {
        // Already sRGB. The `srgb` chunk's rendering intent is not needed for
        // a conversion back to sRGB.
    } else if pil_image_is_png_with_gamma_and_chromaticity(format, mode, info) {
        dequantised = convert_gamma_chromaticity_png_to_srgb(&dequantised, info)
            .unwrap_or(dequantised);
    }

    normalise_pil_image_to_rgb(&dequantised)
}

/// `normalise_numpy_image_to_uint8( numpy_image )`.
pub fn normalise_numpy_image_to_uint8(image: &DynamicImage) -> DynamicImage {
    normalise_image_to_uint8(image)
}

/// `dequantize_freshly_loaded_numpy_image( numpy_image )`.
///
/// The Rust decoder already returns RGB/RGBA data rather than OpenCV's BGR
/// order, so this is the byte-depth normalisation step.
pub fn dequantize_freshly_loaded_numpy_image(image: &DynamicImage) -> DynamicImage {
    normalise_image_to_uint8(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{
        GenericImageView, GrayAlphaImage, Luma, LumaA, Rgb, RgbImage, Rgba, RgbaImage,
    };

    #[test]
    fn sixteen_bit_grayscale_becomes_eight_bit() {
        let image: image::ImageBuffer<Luma<u16>, Vec<u16>> =
            image::ImageBuffer::from_pixel(2, 2, Luma([65535]));

        let normalised = normalise_image_to_uint8(&DynamicImage::ImageLuma16(image));

        assert_eq!(normalised.color(), image::ColorType::L8);
        assert_eq!(normalised.to_luma8().get_pixel(0, 0)[0], 255);
    }

    #[test]
    fn opaque_rgba_loses_its_useless_alpha() {
        let _guard = crate::hydrus_image_colours::TEST_LOCK.lock().unwrap();

        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            4,
            4,
            Rgba([10, 20, 30, 255]),
        ));

        let stripped = strip_out_any_useless_alpha_channel(&image);

        assert_eq!(stripped.color(), image::ColorType::Rgb8);
    }

    #[test]
    fn interesting_alpha_is_kept() {
        let _guard = crate::hydrus_image_colours::TEST_LOCK.lock().unwrap();

        let mut buffer = RgbaImage::from_pixel(10, 10, Rgba([10, 20, 30, 255]));
        buffer.put_pixel(0, 0, Rgba([10, 20, 30, 128]));

        let image = DynamicImage::ImageRgba8(buffer);
        let stripped = strip_out_any_useless_alpha_channel(&image);

        assert_eq!(stripped.color(), image::ColorType::Rgba8);
    }

    #[test]
    fn grayscale_alpha_is_stripped_to_plain_grayscale() {
        let image = DynamicImage::ImageLumaA8(GrayAlphaImage::from_pixel(
            4,
            4,
            LumaA([100, 255]),
        ));

        let stripped = strip_out_any_alpha_channel(&image);

        assert_eq!(stripped.color(), image::ColorType::L8);
    }

    #[test]
    fn rgb_is_untouched() {
        let image = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            4,
            4,
            image::Rgb([1, 2, 3]),
        ));

        let stripped = strip_out_any_alpha_channel(&image);

        assert_eq!(stripped.color(), image::ColorType::Rgb8);
    }

    fn png_srgb_info() -> MetadataMap {
        let mut info = MetadataMap::new();

        info.insert("gamma".to_string(), MetadataValue::Float(0.45455));
        info.insert(
            "chromaticity".to_string(),
            MetadataValue::List(vec![
                MetadataValue::Float(0.3127),
                MetadataValue::Float(0.3290),
                MetadataValue::Float(0.6400),
                MetadataValue::Float(0.3300),
                MetadataValue::Float(0.3000),
                MetadataValue::Float(0.6000),
                MetadataValue::Float(0.1500),
                MetadataValue::Float(0.0600),
            ]),
        );

        info
    }

    #[test]
    fn png_gamma_and_chromaticity_are_recognised() {
        let info = png_srgb_info();

        assert!(pil_image_is_png_with_gamma_and_chromaticity(
            "PNG", "RGB", &info
        ));
        assert!(!pil_image_is_png_with_gamma_and_chromaticity(
            "JPEG", "RGB", &info
        ));
        assert_eq!(get_linear_gamma_from_pil_png(&info), 0.45455);

        let mut srgb_info = MetadataMap::new();
        srgb_info.insert("srgb".to_string(), MetadataValue::Int(0));

        assert!(pil_image_is_png_with_srgb("PNG", &srgb_info));
    }

    #[test]
    fn generated_icc_profile_has_the_expected_header() {
        let profile =
            generate_icc_profile_bytes_from_gamma_and_chromaticity_png(&png_srgb_info())
                .unwrap();

        assert!(profile.len() > 128);
        assert_eq!(profile.len() % 4, 0);
        assert_eq!(&profile[36..40], b"acsp");
    }

    #[test]
    fn gamma_chromaticity_conversion_keeps_an_srgb_colour_close() {
        let source = DynamicImage::ImageRgb8(RgbImage::from_pixel(
            2,
            2,
            Rgb([200, 100, 50]),
        ));

        let converted =
            convert_gamma_chromaticity_png_to_srgb(&source, &png_srgb_info()).unwrap();
        let pixel = converted.to_rgb8().get_pixel(0, 0).0;

        // The source uses a pure 2.2 power curve while sRGB's transfer function
        // has a linear toe, so dark values shift by a few levels by design.
        assert!((i32::from(pixel[0]) - 200).abs() <= 5);
        assert!((i32::from(pixel[1]) - 100).abs() <= 5);
        assert!((i32::from(pixel[2]) - 50).abs() <= 5);
    }

    #[test]
    fn gamma_chromaticity_conversion_preserves_alpha() {
        let source = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            1,
            1,
            Rgba([200, 100, 50, 123]),
        ));

        let converted =
            convert_gamma_chromaticity_png_to_srgb(&source, &png_srgb_info()).unwrap();

        assert_eq!(converted.color(), image::ColorType::Rgba8);
        assert_eq!(converted.to_rgba8().get_pixel(0, 0)[3], 123);
    }

    #[test]
    fn dequantize_to_rgb_drops_a_grayscale_mode() {
        let image = DynamicImage::ImageLuma8(image::GrayImage::from_pixel(
            2,
            2,
            Luma([42]),
        ));

        let converted = normalise_pil_image_to_rgb(&image);

        assert_eq!(converted.color(), image::ColorType::Rgb8);
        assert_eq!(converted.to_rgb8().get_pixel(0, 0).0, [42, 42, 42]);
    }

    #[test]
    fn exif_rotation_is_skipped_for_png_and_applied_elsewhere() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(
            3,
            2,
            Rgb([1, 2, 3]),
        ));

        assert_eq!(
            rotate_exif_pil_image(&image, "PNG", Some(6)).dimensions(),
            (3, 2)
        );
        assert_eq!(
            rotate_exif_pil_image(&image, "JPEG", Some(6)).dimensions(),
            (2, 3)
        );
        assert_eq!(
            rotate_exif_pil_image(&image, "JPEG", Some(2)).dimensions(),
            (3, 2)
        );
    }
}
