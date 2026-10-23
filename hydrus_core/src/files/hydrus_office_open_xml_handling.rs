//! Rust port of `hydrus/core/files/HydrusOfficeOpenXMLHandling.py`.
//!
//! Office Open XML files are zips. The `[Content_Types].xml` part tells us
//! whether a document is DOCX/XLSX/PPTX, `docProps/app.xml` carries a word
//! count, and `docProps/thumbnail.jpeg` is an embedded preview image.

use std::io::Cursor;
use std::path::Path;

use image::imageops::FilterType;
use image::DynamicImage;
use roxmltree::Document;

use crate::files::hydrus_archive_handling;
use crate::hydrus_constants::Mime;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

pub const DOCX_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
pub const XLSX_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
pub const PPTX_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";

pub const PPTX_ASSUMED_DPI: f64 = 300.0;
pub const EMU_PER_INCH: f64 = 914_400.0;

const PRESENTATIONML_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/presentationml/2006/main";
const EXTENDED_PROPERTIES_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties";

fn read_zip_text(path: &Path, entry: &str) -> HydrusResult<String> {
    hydrus_archive_handling::get_zip_as_path(path.to_path_buf(), entry).read_text()
}

fn parse_document(text: &str) -> HydrusResult<Document<'_>> {
    Document::parse(text)
        .map_err(|error| HydrusException::new(ErrorKind::DamagedOrUnusualFile, format!("Bad XML: {error}")))
}

fn has_override(document: &Document<'_>, part_name: &str, content_type: &str) -> bool {
    document.descendants().any(|node| {
        node.tag_name().name() == "Override"
            && node.attribute("PartName") == Some(part_name)
            && node.attribute("ContentType") == Some(content_type)
    })
}

fn has_default(document: &Document<'_>, extension: &str, content_type: &str) -> bool {
    document.descendants().any(|node| {
        node.tag_name().name() == "Default"
            && node.attribute("Extension") == Some(extension)
            && node.attribute("ContentType") == Some(content_type)
    })
}

/// `mime_from_microsoft_open_xml_document( path )`
pub fn mime_from_microsoft_open_xml_document(path: impl AsRef<Path>) -> Option<Mime> {
    let text = read_zip_text(path.as_ref(), "[Content_Types].xml").ok()?;
    let document = parse_document(&text).ok()?;

    if has_override(&document, "/word/document.xml", DOCX_CONTENT_TYPE) {
        return Some(Mime::Docx);
    }

    if has_override(&document, "/xl/workbook.xml", XLSX_CONTENT_TYPE) {
        return Some(Mime::Xlsx);
    }

    if has_override(&document, "/ppt/presentation.xml", PPTX_CONTENT_TYPE) {
        return Some(Mime::Pptx);
    }

    if has_default(&document, "xml", DOCX_CONTENT_TYPE) {
        return Some(Mime::Docx);
    }

    if has_default(&document, "xml", XLSX_CONTENT_TYPE) {
        return Some(Mime::Xlsx);
    }

    if has_default(&document, "xml", PPTX_CONTENT_TYPE) {
        return Some(Mime::Pptx);
    }

    None
}

/// `generate_thumbnail_numpy_from_office_path( path, target_resolution )`
///
/// Rust has no numpy, so the decoded/resized image is returned directly.
pub fn generate_thumbnail_from_office_path(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
) -> HydrusResult<DynamicImage> {
    let bytes = hydrus_archive_handling::get_zip_as_path(
        path.as_ref().to_path_buf(),
        "docProps/thumbnail.jpeg",
    )
    .read_bytes()
    .map_err(|error| {
        if error.is_a(ErrorKind::FileMissing) {
            HydrusException::new(ErrorKind::NoThumbnailFile, "No thumbnail.jpeg file!")
        } else {
            error
        }
    })?;

    let image = image::load_from_memory(&bytes).map_err(|error| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            format!("Could not decode the Office thumbnail: {error}"),
        )
    })?;

    Ok(image.resize_exact(target_resolution.0, target_resolution.1, FilterType::Lanczos3))
}

/// `power_point_resolution( path )`
pub fn power_point_resolution(path: impl AsRef<Path>) -> HydrusResult<(u32, u32)> {
    let text = read_zip_text(path.as_ref(), "ppt/presentation.xml")?;
    let document = parse_document(&text)?;

    let slide_size = document.descendants().find(|node| {
        node.tag_name().name() == "sldSz"
            && node.tag_name().namespace() == Some(PRESENTATIONML_NAMESPACE)
    });

    let Some(slide_size) = slide_size else {
        return Err(HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "No PowerPoint slide size!",
        ));
    };

    let parse_attribute = |name: &str| -> HydrusResult<f64> {
        slide_size
            .attribute(name)
            .ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::DamagedOrUnusualFile,
                    format!("PowerPoint slide size had no {name}!"),
                )
            })?
            .parse::<f64>()
            .map_err(|error| {
                HydrusException::new(
                    ErrorKind::DamagedOrUnusualFile,
                    format!("Bad PowerPoint slide size: {error}"),
                )
            })
    };

    let x_emu = parse_attribute("cx")?;
    let y_emu = parse_attribute("cy")?;

    let pixel_per_emu = PPTX_ASSUMED_DPI / EMU_PER_INCH;

    Ok((
        (x_emu * pixel_per_emu).round() as u32,
        (y_emu * pixel_per_emu).round() as u32,
    ))
}

/// `office_document_word_count( path )`
pub fn office_document_word_count(path: impl AsRef<Path>) -> HydrusResult<i64> {
    let text = read_zip_text(path.as_ref(), "docProps/app.xml")?;
    let document = parse_document(&text)?;

    let words = document.descendants().find(|node| {
        node.tag_name().name() == "Words"
            && node.tag_name().namespace() == Some(EXTENDED_PROPERTIES_NAMESPACE)
    });

    let Some(words) = words else {
        return Err(HydrusException::new(
            ErrorKind::DataMissing,
            "No Office word count!",
        ));
    };

    words
        .text()
        .unwrap_or_default()
        .trim()
        .parse::<i64>()
        .map_err(|error| {
            HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                format!("Bad Office word count: {error}"),
            )
        })
}

/// `get_pptx_info( path )`
pub fn get_pptx_info(
    path: impl AsRef<Path>,
) -> (Option<i64>, (Option<u32>, Option<u32>)) {
    let resolution = power_point_resolution(path.as_ref())
        .map(|(width, height)| (Some(width), Some(height)))
        .unwrap_or((None, None));

    let num_words = office_document_word_count(path.as_ref()).ok();

    (num_words, resolution)
}

/// `get_docx_info( path )`
pub fn get_docx_info(path: impl AsRef<Path>) -> Option<i64> {
    office_document_word_count(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn unique_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "hydrus_ooxml_{}_{}_{}.zip",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn build_zip(label: &str, entries: &[(&str, &[u8])]) -> PathBuf {
        let path = unique_path(label);

        let file = std::fs::File::create(&path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        for (name, data) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }

        writer.finish().unwrap();

        path
    }

    fn word_count_app(words: &str) -> String {
        format!(
            r#"<?xml version="1.0"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">
  <Words>{words}</Words>
</Properties>"#
        )
    }

    #[test]
    fn override_content_types_identify_the_format() {
        let docx = format!(
            r#"<?xml version="1.0"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Override PartName="/word/document.xml" ContentType="{DOCX_CONTENT_TYPE}"/>
</Types>"#
        );

        let path = build_zip("docx", &[("[Content_Types].xml", docx.as_bytes())]);

        assert_eq!(
            mime_from_microsoft_open_xml_document(&path),
            Some(Mime::Docx)
        );

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn default_content_types_are_a_fallback() {
        let pptx = format!(
            r#"<?xml version="1.0"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="{PPTX_CONTENT_TYPE}"/>
</Types>"#
        );

        let path = build_zip("pptx", &[("[Content_Types].xml", pptx.as_bytes())]);

        assert_eq!(
            mime_from_microsoft_open_xml_document(&path),
            Some(Mime::Pptx)
        );

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn powerpoint_slide_size_is_converted_from_emu() {
        let presentation = r#"<?xml version="1.0"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#;

        let path = build_zip("slide", &[("ppt/presentation.xml", presentation.as_bytes())]);

        assert_eq!(power_point_resolution(&path).unwrap(), (3000, 2250));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn word_count_is_read_from_app_xml() {
        let app = word_count_app("1234");
        let path = build_zip("words", &[("docProps/app.xml", app.as_bytes())]);

        assert_eq!(office_document_word_count(&path).unwrap(), 1234);
        assert_eq!(get_docx_info(&path), Some(1234));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn missing_metadata_is_swallowed_by_the_combined_helpers() {
        let path = build_zip("empty", &[("mimetype", b"nothing")]);

        assert_eq!(office_document_word_count(&path).unwrap_err().kind, ErrorKind::FileMissing);
        assert_eq!(get_docx_info(&path), None);
        assert_eq!(get_pptx_info(&path), (None, (None, None)));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn thumbnail_is_decoded_and_resized() {
        let source = image::RgbaImage::from_pixel(8, 8, image::Rgba([10, 20, 30, 255]));
        let mut thumbnail = Cursor::new(Vec::new());

        image::DynamicImage::ImageRgba8(source)
            .write_to(&mut thumbnail, image::ImageFormat::Png)
            .unwrap();

        let path = build_zip(
            "thumb",
            &[("docProps/thumbnail.jpeg", thumbnail.get_ref().as_slice())],
        );

        let image = generate_thumbnail_from_office_path(&path, (4, 4)).unwrap();

        assert_eq!(image.width(), 4);
        assert_eq!(image.height(), 4);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn missing_thumbnail_is_a_no_thumbnail_error() {
        let path = build_zip("no_thumb", &[("mimetype", b"nothing")]);

        let error = generate_thumbnail_from_office_path(&path, (4, 4)).unwrap_err();

        assert!(error.is_a(ErrorKind::NoThumbnailFile));

        std::fs::remove_file(path).unwrap();
    }
}
