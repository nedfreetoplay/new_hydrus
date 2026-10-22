//! Rust port of `hydrus/core/files/images/HydrusImageOpening.py`.
//!
//! The Python module wraps `PILImage.open` so malformed files consistently
//! become `DamagedOrUnusualFileException`s with a useful description. This port
//! does the same for the Rust `image` decoder.

use std::io::{BufRead, Cursor, Seek};
use std::path::Path;

use image::{DynamicImage, ImageReader};

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

fn damaged(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DamagedOrUnusualFile, message)
}

fn description_for_path(path: &Path, human_file_description: Option<&str>) -> String {
    human_file_description
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn decode_without_orientation<R: BufRead + Seek>(
    reader: ImageReader<R>,
    failure_message: &str,
) -> HydrusResult<DynamicImage> {
    // PIL does not apply EXIF orientation in `open`; callers that want it do so
    // explicitly. Keep that behaviour here as well.
    let mut reader = reader;
    reader.no_limits();

    let decoder = reader.into_decoder().map_err(|error| {
        damaged(format!("{failure_message} ({error})"))
    })?;

    DynamicImage::from_decoder(decoder)
        .map_err(|error| damaged(format!("{failure_message} ({error})")))
}

fn path_failure_message(description: &str) -> String {
    format!("Could not load the image at \"{description}\"--it was likely malformed!")
}

/// `raw_open_pil_image( path, human_file_description = None )`
pub fn raw_open_pil_image(
    path: impl AsRef<Path>,
    human_file_description: Option<&str>,
) -> HydrusResult<DynamicImage> {
    let path = path.as_ref();
    let description = description_for_path(path, human_file_description);
    let failure_message = path_failure_message(&description);

    let reader = ImageReader::open(path)
        .map_err(|error| damaged(format!("{failure_message} ({error})")))?
        .with_guessed_format()
        .map_err(|error| damaged(format!("{failure_message} ({error})")))?;

    decode_without_orientation(reader, &failure_message)
}

/// The `typing.BinaryIO` branch of `raw_open_pil_image`.
pub fn raw_open_pil_image_bytes(
    bytes: &[u8],
    human_file_description: Option<&str>,
) -> HydrusResult<DynamicImage> {
    let failure_message = match human_file_description {
        Some(description) => path_failure_message(description),
        None => "Could not load the image, which had no path (so was probably from inside another file?)--it was likely malformed!".to_string(),
    };

    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| damaged(format!("{failure_message} ({error})")))?;

    decode_without_orientation(reader, &failure_message)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use image::{GenericImageView, Rgb, RgbImage};

    use super::*;

    fn png_bytes() -> Vec<u8> {
        let image = RgbImage::from_pixel(7, 5, Rgb([12, 34, 56]));
        let mut bytes = Vec::new();

        DynamicImage::ImageRgb8(image)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();

        bytes
    }

    #[test]
    fn bytes_open_without_requiring_a_path() {
        let image = raw_open_pil_image_bytes(&png_bytes(), None).unwrap();

        assert_eq!(image.dimensions(), (7, 5));
    }

    #[test]
    fn a_path_opens() {
        let path = std::env::temp_dir().join(format!(
            "hydrus_image_opening_{}_{}.png",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        std::fs::File::create(&path)
            .unwrap()
            .write_all(&png_bytes())
            .unwrap();

        let image = raw_open_pil_image(&path, None).unwrap();

        assert_eq!(image.dimensions(), (7, 5));

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn malformed_input_is_a_damaged_file() {
        let error = raw_open_pil_image_bytes(b"not an image", Some("test data"))
            .unwrap_err();

        assert!(error.is_a(ErrorKind::DamagedOrUnusualFile));
        assert!(error.to_string().contains("test data"));

        let error = raw_open_pil_image_bytes(b"still not an image", None).unwrap_err();

        assert!(error.to_string().contains("no path"));
    }
}
