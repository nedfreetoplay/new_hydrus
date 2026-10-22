//! Rust port of `hydrus/core/files/HydrusORAHandling.py`.
//!
//! OpenRaster archives contain `mergedimage.png`, `Thumbnails/thumbnail.png`,
//! and a `stack.xml` with the canvas dimensions.

use std::path::Path;

use image::DynamicImage;
use roxmltree::Document;

use crate::hydrus_archive_handling;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_image_handling;

pub const ORA_FILE_THUMB: &str = "Thumbnails/thumbnail.png";
pub const ORA_FILE_MERGED: &str = "mergedimage.png";
const DOCUMENT_INFO_FILE: &str = "stack.xml";

/// `merged_pil_image_from_ora( path )`
pub fn merged_pil_image_from_ora(path: impl AsRef<Path>) -> HydrusResult<DynamicImage> {
    let bytes = hydrus_archive_handling::get_zip_as_path(path.as_ref().to_path_buf(), ORA_FILE_MERGED)
        .read_bytes()
        .map_err(|error| {
            if error.is_a(ErrorKind::FileMissing) {
                HydrusException::new(
                    ErrorKind::NoRenderFile,
                    format!("Could not read {ORA_FILE_MERGED} from this ora file"),
                )
            } else {
                error
            }
        })?;

    hydrus_image_handling::decode_image_bytes(&bytes)
}

/// `thumbnail_pil_image_from_ora( path )`
pub fn thumbnail_pil_image_from_ora(path: impl AsRef<Path>) -> HydrusResult<DynamicImage> {
    let bytes = hydrus_archive_handling::get_zip_as_path(path.as_ref().to_path_buf(), ORA_FILE_THUMB)
        .read_bytes()
        .map_err(|error| {
            if error.is_a(ErrorKind::FileMissing) {
                HydrusException::new(
                    ErrorKind::NoThumbnailFile,
                    format!("Could not read {ORA_FILE_THUMB} from this ora file"),
                )
            } else {
                error
            }
        })?;

    hydrus_image_handling::decode_image_bytes(&bytes)
}

/// `generate_thumbnail_numpy_from_ora_path( path, target_resolution )`
pub fn generate_thumbnail_numpy_from_ora_path(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
) -> HydrusResult<DynamicImage> {
    let path = path.as_ref();

    let image = merged_pil_image_from_ora(path)
        .or_else(|_| thumbnail_pil_image_from_ora(path))?;

    Ok(hydrus_image_handling::resize_image(
        &image,
        target_resolution,
        None,
    ))
}

/// `get_ora_properties( path )`
pub fn get_ora_properties(path: impl AsRef<Path>) -> HydrusResult<(u32, u32)> {
    let bytes = hydrus_archive_handling::get_zip_as_path(path.as_ref().to_path_buf(), DOCUMENT_INFO_FILE)
        .read_bytes()
        .map_err(|_| {
            HydrusException::new(
                ErrorKind::NoResolutionFile,
                format!(
                    "This ora file had no {DOCUMENT_INFO_FILE} or it contains no resolution!"
                ),
            )
        })?;

    let text = String::from_utf8_lossy(&bytes);

    let document = Document::parse(&text).map_err(|_| {
        HydrusException::new(
            ErrorKind::NoResolutionFile,
            format!(
                "This ora file had no {DOCUMENT_INFO_FILE} or it contains no resolution!"
            ),
        )
    })?;

    let root = document.root_element();

    let width = root
        .attribute("w")
        .and_then(|value| value.parse::<u32>().ok());
    let height = root
        .attribute("h")
        .and_then(|value| value.parse::<u32>().ok());

    match (width, height) {
        (Some(width), Some(height)) => Ok((width, height)),
        _ => Err(HydrusException::new(
            ErrorKind::NoResolutionFile,
            format!(
                "This ora file had no {DOCUMENT_INFO_FILE} or it contains no resolution!"
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    use super::*;

    fn unique_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "hydrus_ora_{}_{}_{}.ora",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn resolution_is_read_from_stack_xml() {
        let path = unique_path("resolution");

        let file = std::fs::File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);

        zip.start_file("stack.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"<image w="800" h="600"><stack/></image>"#)
            .unwrap();
        zip.finish().unwrap();

        assert_eq!(get_ora_properties(&path).unwrap(), (800, 600));

        std::fs::remove_file(path).ok();
    }
}
