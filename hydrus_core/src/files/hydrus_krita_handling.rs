//! Rust port of `hydrus/core/files/HydrusKritaHandling.py`.
//!
//! Krita files are ZIP archives containing `mergedimage.png` and a smaller
//! `preview.png`. Rendering prefers the merged image and falls back to the
//! preview.

use std::path::Path;

use image::DynamicImage;
use roxmltree::Document;

use crate::files::hydrus_archive_handling;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::files::images::hydrus_image_handling;

pub const KRITA_FILE_THUMB: &str = "preview.png";
pub const KRITA_FILE_MERGED: &str = "mergedimage.png";
const DOCUMENT_INFO_FILE: &str = "maindoc.xml";

/// `merged_pil_image_from_kra( path )`
pub fn merged_pil_image_from_kra(path: impl AsRef<Path>) -> HydrusResult<DynamicImage> {
    let bytes = hydrus_archive_handling::get_zip_as_path(path.as_ref().to_path_buf(), KRITA_FILE_MERGED)
        .read_bytes()
        .map_err(|error| {
            if error.is_a(ErrorKind::FileMissing) {
                HydrusException::new(
                    ErrorKind::NoRenderFile,
                    format!("Could not read {KRITA_FILE_MERGED} from this Krita file"),
                )
            } else {
                error
            }
        })?;

    hydrus_image_handling::decode_image_bytes(&bytes)
}

/// `thumbnail_pil_image_from_kra( path )`
pub fn thumbnail_pil_image_from_kra(path: impl AsRef<Path>) -> HydrusResult<DynamicImage> {
    let bytes = hydrus_archive_handling::get_zip_as_path(path.as_ref().to_path_buf(), KRITA_FILE_THUMB)
        .read_bytes()
        .map_err(|error| {
            if error.is_a(ErrorKind::FileMissing) {
                HydrusException::new(
                    ErrorKind::NoThumbnailFile,
                    format!("Could not read {KRITA_FILE_THUMB} from this Krita file"),
                )
            } else {
                error
            }
        })?;

    hydrus_image_handling::decode_image_bytes(&bytes)
}

/// `generate_thumbnail_numpy_from_kra_path( path, target_resolution )`
pub fn generate_thumbnail_numpy_from_kra_path(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
) -> HydrusResult<DynamicImage> {
    let path = path.as_ref();

    let image = merged_pil_image_from_kra(path)
        .or_else(|_| thumbnail_pil_image_from_kra(path))?;

    Ok(hydrus_image_handling::resize_image(
        &image,
        target_resolution,
        None,
    ))
}

/// `get_kra_properties( path )`
pub fn get_kra_properties(path: impl AsRef<Path>) -> HydrusResult<(u32, u32)> {
    let bytes = hydrus_archive_handling::get_zip_as_path(path.as_ref().to_path_buf(), DOCUMENT_INFO_FILE)
        .read_bytes()
        .map_err(|_| {
            HydrusException::new(
                ErrorKind::NoResolutionFile,
                format!(
                    "This krita file had no {DOCUMENT_INFO_FILE} or it contains no resolution!"
                ),
            )
        })?;

    let text = String::from_utf8_lossy(&bytes);

    let document = Document::parse(&text).map_err(|_| {
        HydrusException::new(
            ErrorKind::NoResolutionFile,
            format!(
                "This krita file had no {DOCUMENT_INFO_FILE} or it contains no resolution!"
            ),
        )
    })?;

    let image = document
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "IMAGE")
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::NoResolutionFile,
                format!(
                    "This krita file had no {DOCUMENT_INFO_FILE} or it contains no resolution!"
                ),
            )
        })?;

    let width = image
        .attribute("width")
        .and_then(|value| value.parse::<u32>().ok());
    let height = image
        .attribute("height")
        .and_then(|value| value.parse::<u32>().ok());

    match (width, height) {
        (Some(width), Some(height)) => Ok((width, height)),
        _ => Err(HydrusException::new(
            ErrorKind::NoResolutionFile,
            format!(
                "This krita file had no {DOCUMENT_INFO_FILE} or it contains no resolution!"
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
            "hydrus_krita_{}_{}_{}.kra",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn resolution_is_read_from_maindoc_xml() {
        let path = unique_path("resolution");

        let file = std::fs::File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);

        zip.start_file("maindoc.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            br#"<DOC xmlns="http://www.calligra.org/DTD/krita"><IMAGE width="640" height="480"/></DOC>"#,
        )
        .unwrap();
        zip.finish().unwrap();

        assert_eq!(get_kra_properties(&path).unwrap(), (640, 480));

        std::fs::remove_file(path).ok();
    }
}
