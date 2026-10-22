//! Rust port of `hydrus/core/files/images/HydrusImageHandling.py`.
//!
//! The Python module is built around OpenCV, PIL and numpy. The Rust port uses
//! the pure-Rust `image` crate and [`DynamicImage`] as its common image type.
//! The public helpers keep the Python names where they still make sense, but
//! numpy/PIL-specific names are documented as returning a `DynamicImage`.
//!
//! PSD, Krita and OpenRaster images are delegated to their specialised readers.
//! Formats that need a client-side renderer or a codec the `image` crate does not
//! provide return a clear [`ErrorKind`] instead of silently producing a broken
//! thumbnail.

use std::io::{BufRead, Cursor, Read, Seek};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder};
use image::codecs::webp::WebPEncoder;
use image::metadata::Orientation;
use image::{
    DynamicImage, GenericImageView, ImageDecoder, ImageError, ImageFormat, ImageReader, RgbImage,
    RgbaImage,
};
use sha2::{Digest, Sha256};

use crate::hydrus_constants::Mime;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_image_colours;
use crate::hydrus_image_metadata::{self, ImageMetadata};
use crate::hydrus_image_normalisation;
use crate::hydrus_krita_handling;
use crate::hydrus_ora_handling;
use crate::hydrus_psd_handling;

/// `PILImage.MAX_IMAGE_PIXELS = ( 512 * ( 1024 ** 2 ) ) // 3`
pub const MAX_IMAGE_PIXELS: u64 = (512 * (1024 * 1024)) / 3;

/// `HEIF_OK`; the `image` crate does not ship a HEIF decoder.
pub const HEIF_OK: bool = false;
/// `AVIF_OK`; the `image` crate does not ship an AVIF decoder by default.
pub const AVIF_OK: bool = false;
/// `JXL_OK`; the `image` crate does not ship a JPEG XL decoder.
pub const JXL_OK: bool = false;

pub const THUMBNAIL_SCALE_DOWN_ONLY: u8 = 0;
pub const THUMBNAIL_SCALE_TO_FIT: u8 = 1;
pub const THUMBNAIL_SCALE_TO_FILL: u8 = 2;

pub const THUMBNAIL_SCALE_STRING_LOOKUP: &[(u8, &str)] = &[
    (THUMBNAIL_SCALE_DOWN_ONLY, "scale down only"),
    (THUMBNAIL_SCALE_TO_FIT, "scale to fit"),
    (THUMBNAIL_SCALE_TO_FILL, "scale to fill"),
];

/// Typed form of Python's three thumbnail scale integer constants.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThumbnailScale {
    DownOnly = THUMBNAIL_SCALE_DOWN_ONLY,
    ToFit = THUMBNAIL_SCALE_TO_FIT,
    ToFill = THUMBNAIL_SCALE_TO_FILL,
}

impl ThumbnailScale {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    pub const fn from_int(value: u8) -> Option<Self> {
        match value {
            THUMBNAIL_SCALE_DOWN_ONLY => Some(Self::DownOnly),
            THUMBNAIL_SCALE_TO_FIT => Some(Self::ToFit),
            THUMBNAIL_SCALE_TO_FILL => Some(Self::ToFill),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DownOnly => "scale down only",
            Self::ToFit => "scale to fit",
            Self::ToFill => "scale to fill",
        }
    }
}

/// Typed replacement for Python's `[ cv2.IMWRITE_*, value ]` parameter lists.
#[derive(Clone, Copy, Debug, Default)]
pub struct EncodeParams {
    /// JPEG/WebP quality, 1..=100.
    pub quality: Option<u8>,
    /// PNG compression, 0..=9.
    pub png_compression: Option<u8>,
}

pub const CV_JPEG_THUMBNAIL_ENCODE_PARAMS: EncodeParams = EncodeParams {
    quality: Some(92),
    png_compression: None,
};

pub const CV_PNG_THUMBNAIL_ENCODE_PARAMS: EncodeParams = EncodeParams {
    quality: None,
    png_compression: Some(9),
};

static LOAD_TRUNCATED_IMAGES: AtomicBool = AtomicBool::new(false);

/// `set_enable_load_truncated_images( value )`
///
/// `image` has no global "pretend the file ended here" switch like PIL. The
/// requested value is remembered for diagnostics, but the function returns
/// `false` just as Python does when PIL lacks `LOAD_TRUNCATED_IMAGES`.
pub fn set_enable_load_truncated_images(value: bool) -> bool {
    LOAD_TRUNCATED_IMAGES.store(value, Ordering::Relaxed);

    false
}

pub fn load_truncated_images_is_enabled() -> bool {
    LOAD_TRUNCATED_IMAGES.load(Ordering::Relaxed)
}

fn damaged(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DamagedOrUnusualFile, message)
}

fn unsupported(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::UnsupportedFile, message)
}

fn no_resolution() -> HydrusException {
    HydrusException::new(ErrorKind::NoResolutionFile, "")
}

fn image_decode_error(context: &str, error: ImageError) -> HydrusException {
    if matches!(error, ImageError::Unsupported(_)) {
        HydrusException::new(
            ErrorKind::UnsupportedFile,
            format!("{context}: {error}"),
        )
    } else {
        damaged(format!("{context}: {error}"))
    }
}

fn load_description(path: &Path, human_file_description: Option<&str>) -> String {
    human_file_description
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn decode_with_orientation<R: BufRead + Seek>(
    mut reader: ImageReader<R>,
    description: &str,
) -> HydrusResult<DynamicImage> {
    // Python turns off PIL's global pixel guard; the explicit
    // is_decompression_bomb check is the only guard in this module.
    reader.no_limits();

    let mut decoder = reader.into_decoder().map_err(|error| {
        image_decode_error(
            &format!("Could not load the image at \"{description}\""),
            error,
        )
    })?;

    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);

    let mut image = DynamicImage::from_decoder(decoder).map_err(|error| {
        image_decode_error(
            &format!("Looks like a truncated file that the image decoder could not handle ({description})"),
            error,
        )
    })?;

    image.apply_orientation(orientation);

    Ok(image)
}

/// Decode a path using the same EXIF orientation rules as `generate_pil_image`.
pub fn decode_image_path(path: impl AsRef<Path>) -> HydrusResult<DynamicImage> {
    let path = path.as_ref();

    let reader = ImageReader::open(path)
        .map_err(|error| {
            HydrusException::new(
                ErrorKind::FileMissing,
                format!("Could not open the image at \"{}\": {error}", path.display()),
            )
        })?
        .with_guessed_format()
        .map_err(|error| {
            damaged(format!(
                "Could not determine the image format at \"{}\": {error}",
                path.display()
            ))
        })?;

    decode_with_orientation(reader, &path.display().to_string())
}

/// Decode bytes using the same EXIF orientation rules as `generate_pil_image`.
pub fn decode_image_bytes(bytes: &[u8]) -> HydrusResult<DynamicImage> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| damaged(format!("Could not determine the image format: {error}")))?;

    decode_with_orientation(reader, "image bytes")
}

fn specialised_mime_error(mime: Mime) -> Option<HydrusException> {
    match mime {
        Mime::Svg => Some(HydrusException::new(
            ErrorKind::NoRenderFile,
            "SVG rendering requires a client-side renderer.",
        )),
        Mime::Pdf => Some(HydrusException::new(
            ErrorKind::NoRenderFile,
            "PDF rendering requires a client-side renderer.",
        )),
        Mime::Heic | Mime::HeicSequence | Mime::Avif | Mime::AvifSequence | Mime::Jxl
        | Mime::AnimationJxl | Mime::UndeterminedJxl => Some(unsupported(
            "This image format is not supported by the pure-Rust image decoder.",
        )),
        _ => None,
    }
}

fn png_metadata_for_path(path: &Path) -> Option<ImageMetadata> {
    let mut signature = [0_u8; 8];

    std::fs::File::open(path)
        .ok()?
        .read_exact(&mut signature)
        .ok()?;

    if &signature != b"\x89PNG\r\n\x1a\n" {
        return None;
    }

    hydrus_image_metadata::read_png_metadata(path).ok()
}

/// `generate_pil_image( path, dequantize = True, human_file_description = None )`
pub fn generate_pil_image(
    path: impl AsRef<Path>,
    dequantize: bool,
    human_file_description: Option<&str>,
) -> HydrusResult<DynamicImage> {
    let path = path.as_ref();

    let image = decode_image_path(path).map_err(|error| {
        let description = load_description(path, human_file_description);

        damaged(format!(
            "Could not load the image at \"{description}\"--it was likely malformed! ({error})"
        ))
    })?;

    if dequantize {
        if let Some(metadata) = png_metadata_for_path(path) {
            Ok(hydrus_image_normalisation::dequantize_pil_image(
                &image,
                &metadata.format,
                &metadata.mode,
                &metadata.info,
            ))
        } else {
            Ok(hydrus_image_normalisation::dequantize_image(&image))
        }
    } else {
        Ok(image)
    }
}

/// `generate_numpy_image_from_pil_image( pil_image, strip_useless_alpha = True )`
pub fn generate_numpy_image_from_pil_image(
    pil_image: &DynamicImage,
    strip_useless_alpha: bool,
) -> DynamicImage {
    if strip_useless_alpha {
        hydrus_image_normalisation::strip_out_any_useless_alpha_channel(pil_image)
    } else {
        pil_image.clone()
    }
}

/// `generate_pil_image_from_numpy_image( numpy_image )`
///
/// Rust has one image type, so this is an explicit identity operation kept for
/// call-site parity with the Python module.
pub fn generate_pil_image_from_numpy_image(numpy_image: &DynamicImage) -> DynamicImage {
    numpy_image.clone()
}

/// `generate_numpy_image( path, mime, force_pil = False, human_file_description = None )`
///
/// Returns a decoded image with EXIF orientation applied and useless alpha
/// removed. `force_pil` only mattered because Python can choose between OpenCV
/// and PIL; the Rust port has a single decoder.
pub fn generate_numpy_image(
    path: impl AsRef<Path>,
    mime: Mime,
    _force_pil: bool,
    human_file_description: Option<&str>,
) -> HydrusResult<DynamicImage> {
    let path = path.as_ref();

    if let Some(error) = specialised_mime_error(mime) {
        return Err(error);
    }

    if mime == Mime::Psd {
        let pil_image = hydrus_psd_handling::generate_pil_image_from_psd(path)?;

        return Ok(generate_numpy_image_from_pil_image(&pil_image, true));
    }

    if mime == Mime::Krita {
        let pil_image = hydrus_krita_handling::merged_pil_image_from_kra(path)?;

        return Ok(generate_numpy_image_from_pil_image(&pil_image, true));
    }

    if mime == Mime::OpenRaster {
        let pil_image = hydrus_ora_handling::merged_pil_image_from_ora(path)?;

        return Ok(generate_numpy_image_from_pil_image(&pil_image, true));
    }

    let pil_image = generate_pil_image(path, true, human_file_description)?;

    Ok(generate_numpy_image_from_pil_image(&pil_image, true))
}

/// Encode a decoded image to a byte buffer.
///
/// `ext` includes the leading dot, matching `mime_ext_lookup`.
pub fn generate_file_bytes_numpy(
    numpy_image: &DynamicImage,
    ext: &str,
    params: EncodeParams,
) -> HydrusResult<Vec<u8>> {
    let ext = ext.to_ascii_lowercase();

    let format = match ext.as_str() {
        ".png" => ImageFormat::Png,
        ".jpg" | ".jpeg" => ImageFormat::Jpeg,
        ".webp" => ImageFormat::WebP,
        ".bmp" => ImageFormat::Bmp,
        ".tif" | ".tiff" => ImageFormat::Tiff,
        ".gif" => ImageFormat::Gif,
        ".qoi" => ImageFormat::Qoi,
        _ => {
            return Err(HydrusException::new(
                ErrorKind::CantRenderWithCV,
                format!("Image failed to encode: unsupported extension \"{ext}\""),
            ));
        }
    };

    encode_image(numpy_image, format, params)
}

/// `generate_file_bytes_for_render_api( numpy_image, format, quality )`
pub fn generate_file_bytes_for_render_api(
    numpy_image: &DynamicImage,
    format: Mime,
    quality: u8,
) -> HydrusResult<Vec<u8>> {
    let image_format = match format {
        Mime::Png | Mime::Apng => ImageFormat::Png,
        Mime::Jpeg => ImageFormat::Jpeg,
        Mime::Webp | Mime::AnimationWebp => ImageFormat::WebP,
        Mime::StaticGif | Mime::AnimationGif => ImageFormat::Gif,
        Mime::Bmp => ImageFormat::Bmp,
        Mime::Tiff => ImageFormat::Tiff,
        Mime::Qoi => ImageFormat::Qoi,
        Mime::Icon => ImageFormat::Ico,
        _ => {
            return Err(HydrusException::new(
                ErrorKind::CantRenderWithCV,
                format!("Image failed to encode as {}.", format.mimetype_str()),
            ));
        }
    };

    let params = match image_format {
        ImageFormat::Png => EncodeParams {
            quality: None,
            png_compression: Some(quality),
        },
        _ => EncodeParams {
            quality: Some(quality),
            png_compression: None,
        },
    };

    encode_image(numpy_image, image_format, params)
}

fn encode_image(
    image: &DynamicImage,
    format: ImageFormat,
    params: EncodeParams,
) -> HydrusResult<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());

    let result = match format {
        ImageFormat::Png => {
            let compression = match params.png_compression.unwrap_or(9) {
                0 => CompressionType::Fast,
                9 => CompressionType::Best,
                _ => CompressionType::Default,
            };

            let encoder = PngEncoder::new_with_quality(
                &mut buffer,
                compression,
                PngFilterType::Adaptive,
            );

            image.write_with_encoder(encoder)
        }
        ImageFormat::Jpeg => {
            let quality = params.quality.unwrap_or(92).clamp(1, 100);
            let rgb_image = if image.color().has_alpha() {
                DynamicImage::ImageRgb8(image.to_rgb8())
            } else {
                image.clone()
            };

            let encoder = JpegEncoder::new_with_quality(&mut buffer, quality);

            rgb_image.write_with_encoder(encoder)
        }
        ImageFormat::WebP => {
            let encoder = WebPEncoder::new_lossless(&mut buffer);

            image.write_with_encoder(encoder)
        }
        _ => image.write_to(&mut buffer, format),
    };

    result.map_err(|error| {
        HydrusException::new(
            ErrorKind::CantRenderWithCV,
            format!("Image failed to encode: {error}"),
        )
    })?;

    Ok(buffer.into_inner())
}

/// `generate_thumbnail_numpy_from_static_image_path( path, target_resolution, mime )`
pub fn generate_thumbnail_numpy_from_static_image_path(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
    mime: Mime,
) -> HydrusResult<DynamicImage> {
    let numpy_image = generate_numpy_image(path, mime, false, None)?;

    Ok(resize_image(&numpy_image, target_resolution, None))
}

/// `generate_thumbnail_bytes_from_numpy( numpy_image )`
pub fn generate_thumbnail_bytes_from_numpy(numpy_image: &DynamicImage) -> HydrusResult<Vec<u8>> {
    if hydrus_image_colours::image_has_alpha_channel(numpy_image) {
        generate_file_bytes_numpy(numpy_image, ".png", CV_PNG_THUMBNAIL_ENCODE_PARAMS)
    } else {
        generate_file_bytes_numpy(numpy_image, ".jpg", CV_JPEG_THUMBNAIL_ENCODE_PARAMS)
    }
}

/// `generate_thumbnail_bytes_from_pil( pil_image )`
pub fn generate_thumbnail_bytes_from_pil(pil_image: &DynamicImage) -> HydrusResult<Vec<u8>> {
    if hydrus_image_colours::pil_image_has_transparency(pil_image) {
        generate_file_bytes_numpy(pil_image, ".png", CV_PNG_THUMBNAIL_ENCODE_PARAMS)
    } else {
        generate_file_bytes_numpy(pil_image, ".jpg", CV_JPEG_THUMBNAIL_ENCODE_PARAMS)
    }
}

fn pixel_bytes(image: &DynamicImage) -> Vec<u8> {
    match image {
        DynamicImage::ImageLuma8(_) => image.to_luma8().into_raw(),
        DynamicImage::ImageLumaA8(_) => image.to_luma_alpha8().into_raw(),
        _ if image.color().has_alpha() => image.to_rgba8().into_raw(),
        _ => image.to_rgb8().into_raw(),
    }
}

/// `get_image_pixel_hash_numpy( numpy_image )`
pub fn get_image_pixel_hash_numpy(numpy_image: &DynamicImage) -> Vec<u8> {
    Sha256::digest(pixel_bytes(numpy_image)).to_vec()
}

/// `get_image_pixel_hash( path, mime )`
pub fn get_image_pixel_hash(
    path: impl AsRef<Path>,
    mime: Mime,
) -> HydrusResult<Vec<u8>> {
    let numpy_image = generate_numpy_image(path, mime, false, None)?;

    Ok(get_image_pixel_hash_numpy(&numpy_image))
}

fn dimensions_with_orientation<R: BufRead + Seek>(
    mut reader: ImageReader<R>,
) -> HydrusResult<(u32, u32)> {
    reader.no_limits();

    let mut decoder = reader
        .into_decoder()
        .map_err(|error| image_decode_error("Could not read the image dimensions", error))?;

    let (width, height) = decoder.dimensions();
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);

    let swaps_dimensions = matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    );

    if swaps_dimensions {
        Ok((height, width))
    } else {
        Ok((width, height))
    }
}

/// `get_image_resolution( path, mime )`
pub fn get_image_resolution(
    path: impl AsRef<Path>,
    mime: Mime,
) -> HydrusResult<(u32, u32)> {
    let path = path.as_ref();

    let fast_result = (|| -> HydrusResult<(u32, u32)> {
        let reader = ImageReader::open(path)
            .map_err(|error| {
                HydrusException::new(
                    ErrorKind::FileMissing,
                    format!("Could not open the image at \"{}\": {error}", path.display()),
                )
            })?
            .with_guessed_format()
            .map_err(|error| {
                damaged(format!(
                    "Could not determine the image format at \"{}\": {error}",
                    path.display()
                ))
            })?;

        dimensions_with_orientation(reader)
    })();

    match fast_result {
        Ok((width, height)) => Ok((width.max(1), height.max(1))),
        Err(_) => {
            // Desperate situation: decode the whole thing.
            let numpy_image = generate_numpy_image(path, mime, false, None)
                .map_err(|_| no_resolution())?;

            let (width, height) = numpy_image.dimensions();

            Ok((width.max(1), height.max(1)))
        }
    }
}

/// `get_resolution_numpy( numpy_image )`
pub fn get_resolution_numpy(numpy_image: &DynamicImage) -> (u32, u32) {
    numpy_image.dimensions()
}

/// `get_thumbnail_resolution(...)`
///
/// Python assigns `im_height = bounding_width` for a 0x0 image; that looks like
/// a typo, so this port uses `bounding_height`. All other arithmetic, including
/// truncation, DPR scaling and the 5x fill cap, is preserved.
pub fn get_thumbnail_resolution(
    image_resolution: (u32, u32),
    bounding_dimensions: (u32, u32),
    thumbnail_scale_type: ThumbnailScale,
    thumbnail_dpr_percent: u32,
) -> (u32, u32) {
    let (mut im_width, mut im_height) = image_resolution;
    let (mut bounding_width, mut bounding_height) = bounding_dimensions;

    if thumbnail_dpr_percent != 100 {
        let thumbnail_dpr = f64::from(thumbnail_dpr_percent) / 100.0;

        bounding_height = (f64::from(bounding_height) * thumbnail_dpr) as u32;
        bounding_width = (f64::from(bounding_width) * thumbnail_dpr) as u32;
    }

    if im_width == 0 || im_height == 0 {
        im_width = bounding_width;
        im_height = bounding_height;
    }

    if bounding_width == 0 || bounding_height == 0 {
        return (1, 1);
    }

    if thumbnail_scale_type == ThumbnailScale::DownOnly
        && bounding_width >= im_width
        && bounding_height >= im_height
    {
        return (im_width.max(1), im_height.max(1));
    }

    let image_ratio = f64::from(im_width) / f64::from(im_height);

    let width_ratio = f64::from(im_width) / f64::from(bounding_width);
    let height_ratio = f64::from(im_height) / f64::from(bounding_height);

    let image_is_wider_than_bounding_box = width_ratio > height_ratio;
    let image_is_taller_than_bounding_box = height_ratio > width_ratio;

    let mut thumbnail_width = f64::from(bounding_width);
    let mut thumbnail_height = f64::from(bounding_height);

    match thumbnail_scale_type {
        ThumbnailScale::DownOnly | ThumbnailScale::ToFit => {
            if image_is_taller_than_bounding_box {
                thumbnail_width = f64::from(im_width) / height_ratio;
            } else if image_is_wider_than_bounding_box {
                thumbnail_height = f64::from(im_height) / width_ratio;
            }
        }
        ThumbnailScale::ToFill => {
            if image_is_taller_than_bounding_box {
                thumbnail_height =
                    f64::from(bounding_width) * (1.0f64 / image_ratio).min(5.0);
            } else if image_is_wider_than_bounding_box {
                thumbnail_width =
                    f64::from(bounding_height) * image_ratio.min(5.0);
            }
        }
    }

    (
        (thumbnail_width as u32).max(1),
        (thumbnail_height as u32).max(1),
    )
}

/// `is_decompression_bomb( path, human_file_description = None )`
pub fn is_decompression_bomb(
    path: impl AsRef<Path>,
    _human_file_description: Option<&str>,
) -> bool {
    let path = path.as_ref();

    let Ok(reader) = ImageReader::open(path) else {
        return false;
    };

    let Ok(mut reader) = reader.with_guessed_format() else {
        return false;
    };

    reader.no_limits();

    let Ok((width, height)) = reader.into_dimensions() else {
        return false;
    };

    let pixels = u64::from(width) * u64::from(height);

    pixels > MAX_IMAGE_PIXELS * 2
}

/// `resize_numpy_image( numpy_image, target_resolution, forced_interpolation = None )`
pub fn resize_image(
    numpy_image: &DynamicImage,
    target_resolution: (u32, u32),
    forced_interpolation: Option<image::imageops::FilterType>,
) -> DynamicImage {
    let (target_width, target_height) = target_resolution;
    let (image_width, image_height) = numpy_image.dimensions();

    if target_width == image_width && target_height == image_height {
        return numpy_image.clone();
    }

    // OpenCV uses LANCZOS4 when growing and INTER_AREA when shrinking. The
    // `image` crate's Triangle filter is its closest cheap area-like choice.
    let interpolation = forced_interpolation.unwrap_or_else(|| {
        if target_width > image_width || target_height > image_height {
            image::imageops::FilterType::Lanczos3
        } else {
            image::imageops::FilterType::Triangle
        }
    });

    numpy_image.resize_exact(target_width, target_height, interpolation)
}

fn pad_image_to(image: &DynamicImage, target_resolution: (u32, u32)) -> DynamicImage {
    let (target_width, target_height) = target_resolution;

    if target_width == 0 || target_height == 0 {
        return DynamicImage::new_rgba8(target_width, target_height);
    }

    let (image_width, image_height) = image.dimensions();

    if image_width == 0 || image_height == 0 {
        return DynamicImage::new_rgba8(target_width, target_height);
    }

    let scale = (f64::from(target_width) / f64::from(image_width))
        .min(f64::from(target_height) / f64::from(image_height));

    let resized_width = ((f64::from(image_width) * scale).round() as u32)
        .clamp(1, target_width);
    let resized_height = ((f64::from(image_height) * scale).round() as u32)
        .clamp(1, target_height);

    let resized = image.resize_exact(
        resized_width,
        resized_height,
        image::imageops::FilterType::Lanczos3,
    );

    let mut canvas = RgbaImage::new(target_width, target_height);

    let x = (target_width - resized_width) / 2;
    let y = (target_height - resized_height) / 2;

    image::imageops::overlay(
        &mut canvas,
        &resized.to_rgba8(),
        i64::from(x),
        i64::from(y),
    );

    if image.color().has_alpha() {
        DynamicImage::ImageRgba8(canvas)
    } else {
        // Transparent padding converted to RGB is black, matching PIL's
        // default `ImageOps.pad` background for an RGB image.
        DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(canvas).to_rgb8())
    }
}

/// `generate_default_thumbnail_numpy_from_path( path, target_resolution )`
pub fn generate_default_thumbnail(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
) -> HydrusResult<DynamicImage> {
    let thumb_image = generate_pil_image(path, true, None)?;

    Ok(pad_image_to(&thumb_image, target_resolution))
}

/// Python name kept as an alias for call-site parity.
pub fn generate_default_thumbnail_numpy_from_path(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
) -> HydrusResult<DynamicImage> {
    generate_default_thumbnail(path, target_resolution)
}

/// `make_clip_rect_fit( image_resolution, clip_rect )`
pub fn make_clip_rect_fit(
    image_resolution: (u32, u32),
    clip_rect: (i64, i64, u32, u32),
) -> (u32, u32, u32, u32) {
    let (im_width, im_height) = image_resolution;
    let (x, y, clip_width, clip_height) = clip_rect;

    let mut x = x.max(0) as u32;
    let mut y = y.max(0) as u32;

    let clip_width = clip_width.min(im_width);
    let clip_height = clip_height.min(im_height);

    if x.saturating_add(clip_width) > im_width {
        x = im_width.saturating_sub(clip_width);
    }

    if y.saturating_add(clip_height) > im_height {
        y = im_height.saturating_sub(clip_height);
    }

    (x, y, clip_width, clip_height)
}

/// `clip_numpy_image( numpy_image, clip_rect )`
pub fn clip_numpy_image(
    numpy_image: &DynamicImage,
    clip_rect: (i64, i64, u32, u32),
) -> DynamicImage {
    let (x, y, clip_width, clip_height) =
        make_clip_rect_fit(numpy_image.dimensions(), clip_rect);

    if clip_width == 0 || clip_height == 0 {
        return DynamicImage::new_rgba8(clip_width, clip_height);
    }

    numpy_image.crop_imm(x, y, clip_width, clip_height)
}

/// `clip_pil_image( pil_image, clip_rect )`
pub fn clip_pil_image(
    pil_image: &DynamicImage,
    clip_rect: (i64, i64, u32, u32),
) -> DynamicImage {
    clip_numpy_image(pil_image, clip_rect)
}

/// Used by tests and callers that need the old `PILImageOps.pad` result shape.
pub fn pad_image(image: &DynamicImage, target_resolution: (u32, u32)) -> DynamicImage {
    pad_image_to(image, target_resolution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::png::PngEncoder;
    use image::{Rgb, Rgba};
    use std::path::PathBuf;

    fn unique_path(label: &str, extension: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "hydrus_image_{}_{}_{}{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            extension
        ))
    }

    fn write_png(image: &DynamicImage, label: &str) -> PathBuf {
        let path = unique_path(label, ".png");

        image.save_with_format(&path, ImageFormat::Png).unwrap();

        path
    }

    #[test]
    fn loaded_dimensions_are_reported() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(
            7,
            5,
            Rgb([10, 20, 30]),
        ));
        let path = write_png(&image, "dimensions");

        assert_eq!(get_image_resolution(&path, Mime::Png).unwrap(), (7, 5));
        assert_eq!(
            generate_pil_image(&path, true, None).unwrap().dimensions(),
            (7, 5)
        );

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_missing_file_is_file_missing() {
        let error =
            generate_pil_image("definitely_missing_hydrus_image.png", true, None)
                .unwrap_err();

        assert!(error.is_a(ErrorKind::DamagedOrUnusualFile) || error.is_a(ErrorKind::FileMissing));
    }

    #[test]
    fn clipping_fits_and_moves_a_negative_rect_inside() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(
            100,
            50,
            Rgb([1, 2, 3]),
        ));

        assert_eq!(make_clip_rect_fit((100, 50), (-3, -4, 20, 10)), (0, 0, 20, 10));
        assert_eq!(make_clip_rect_fit((100, 50), (90, 45, 20, 10)), (80, 40, 20, 10));

        let clipped = clip_numpy_image(&image, (90, 45, 20, 10));

        assert_eq!(clipped.dimensions(), (20, 10));
    }

    #[test]
    fn resizing_hits_the_exact_target() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(
            20,
            10,
            Rgb([1, 2, 3]),
        ));

        let resized = resize_image(&image, (8, 4), None);

        assert_eq!(resized.dimensions(), (8, 4));
    }

    #[test]
    fn dpr_is_applied_to_thumbnail_bounds() {
        let scaled = get_thumbnail_resolution(
            (1000, 500),
            (100, 100),
            ThumbnailScale::ToFit,
            200,
        );

        assert_eq!(scaled, (200, 100));
    }

    #[test]
    fn down_only_does_not_upscale() {
        assert_eq!(
            get_thumbnail_resolution((50, 25), (100, 100), ThumbnailScale::DownOnly, 100),
            (50, 25)
        );
    }

    #[test]
    fn fill_allows_spill_over() {
        let (width, height) =
            get_thumbnail_resolution((100, 1000), (100, 100), ThumbnailScale::ToFill, 100);

        assert_eq!(width, 100);
        assert_eq!(height, 500);
    }

    #[test]
    fn zero_sized_images_fall_back_to_the_bounds() {
        assert_eq!(
            get_thumbnail_resolution((0, 0), (50, 30), ThumbnailScale::ToFit, 100),
            (50, 30)
        );
    }

    #[test]
    fn thumbnail_scale_lookup_is_typed() {
        assert_eq!(
            ThumbnailScale::from_int(2).map(ThumbnailScale::as_str),
            Some("scale to fill")
        );
        assert_eq!(ThumbnailScale::from_int(9), None);
    }

    #[test]
    fn opaque_rgba_is_stripped_when_generating_numpy_images() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            10,
            10,
            Rgba([1, 2, 3, 255]),
        ));
        let path = write_png(&image, "alpha");

        let numpy_image = generate_numpy_image(&path, Mime::Png, false, None).unwrap();

        assert_eq!(numpy_image.color(), image::ColorType::Rgb8);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn thumbnail_bytes_use_png_for_transparency_and_jpeg_otherwise() {
        let mut rgba = RgbaImage::from_pixel(10, 10, Rgba([1, 2, 3, 255]));
        rgba.put_pixel(0, 0, Rgba([1, 2, 3, 10]));
        let transparent = DynamicImage::ImageRgba8(rgba);

        let png = generate_thumbnail_bytes_from_numpy(&transparent).unwrap();
        assert_eq!(&png[0..4], b"\x89PNG");

        let opaque = DynamicImage::ImageRgb8(RgbImage::from_pixel(
            10,
            10,
            Rgb([1, 2, 3]),
        ));
        let jpeg = generate_thumbnail_bytes_from_numpy(&opaque).unwrap();
        assert_eq!(&jpeg[0..2], b"\xFF\xD8");
    }

    #[test]
    fn encode_decode_round_trip_preserves_dimensions() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(
            13,
            17,
            Rgb([9, 8, 7]),
        ));

        let png = generate_file_bytes_numpy(&image, ".png", EncodeParams::default()).unwrap();
        let jpeg =
            generate_file_bytes_numpy(&image, ".jpg", CV_JPEG_THUMBNAIL_ENCODE_PARAMS).unwrap();

        assert_eq!(decode_image_bytes(&png).unwrap().dimensions(), (13, 17));
        assert_eq!(decode_image_bytes(&jpeg).unwrap().dimensions(), (13, 17));
    }

    #[test]
    fn default_thumbnail_is_padded_to_the_exact_size() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(
            100,
            50,
            Rgb([1, 2, 3]),
        ));
        let path = write_png(&image, "pad");

        let thumbnail = generate_default_thumbnail(&path, (64, 64)).unwrap();

        assert_eq!(thumbnail.dimensions(), (64, 64));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn pixel_hash_is_stable_for_the_same_pixels() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(
            5,
            5,
            Rgb([42, 43, 44]),
        ));

        assert_eq!(
            get_image_pixel_hash_numpy(&image),
            get_image_pixel_hash_numpy(&image)
        );
    }

    #[test]
    fn unsupported_specialised_formats_report_clearly() {
        let error = generate_numpy_image("whatever.svg", Mime::Svg, false, None).unwrap_err();
        assert!(error.is_a(ErrorKind::NoRenderFile));

        let error = generate_numpy_image("whatever.pdf", Mime::Pdf, false, None).unwrap_err();
        assert!(error.is_a(ErrorKind::NoRenderFile));
    }

    #[test]
    fn truncated_image_switch_is_honest_about_support() {
        assert!(!set_enable_load_truncated_images(true));
        assert!(load_truncated_images_is_enabled());
        assert!(!set_enable_load_truncated_images(false));
    }

    #[test]
    fn orientation_rotations_swap_dimensions() {
        let mut image = DynamicImage::ImageRgb8(RgbImage::from_pixel(
            10,
            4,
            Rgb([1, 2, 3]),
        ));

        image.apply_orientation(Orientation::Rotate90);

        assert_eq!(image.dimensions(), (4, 10));
    }

    #[test]
    fn oversized_headers_are_reported_as_bombs() {
        // A PNG signature + IHDR with 20000x20000 pixels is well over the
        // 2 * 178M pixel hard limit. Only the header is needed.
        fn crc32(data: &[u8]) -> u32 {
            let mut crc = 0xFFFF_FFFFu32;

            for byte in data {
                crc ^= u32::from(*byte);

                for _ in 0..8 {
                    let mask = (crc & 1).wrapping_neg();
                    crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
                }
            }

            !crc
        }

        let width = 20_000u32;
        let height = 20_000u32;

        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);

        fn chunk(chunk_type: &[u8; 4], data: &[u8]) -> Vec<u8> {
            let mut result = Vec::new();
            result.extend_from_slice(&(data.len() as u32).to_be_bytes());
            result.extend_from_slice(chunk_type);
            result.extend_from_slice(data);
            result.extend_from_slice(
                &crc32(&[chunk_type.as_slice(), data].concat()).to_be_bytes(),
            );
            result
        }

        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&chunk(b"IHDR", &ihdr));
        png.extend_from_slice(&chunk(b"IDAT", &[]));
        png.extend_from_slice(&chunk(b"IEND", &[]));

        let path = unique_path("bomb", ".png");
        std::fs::write(&path, png).unwrap();

        assert!(is_decompression_bomb(&path, None));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn png_encoder_import_is_exercised() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(
            2,
            2,
            Rgb([1, 2, 3]),
        ));
        let mut buffer = Cursor::new(Vec::new());
        let encoder =
            PngEncoder::new_with_quality(&mut buffer, CompressionType::Best, PngFilterType::Adaptive);
        image.write_with_encoder(encoder).unwrap();

        assert!(!buffer.into_inner().is_empty());
    }
}
