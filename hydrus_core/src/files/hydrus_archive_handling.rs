//! Rust port of `hydrus/core/files/HydrusArchiveHandling.py`.
//!
//! ZIP containers are used by EPUB, CBZ, Procreate, ORA/Krita and the Office
//! Open XML family. The helpers here keep the Python module's forgiving
//! heuristics: find the first image, understand the EPUB cover metadata, and
//! decide whether a zip is a comic archive.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use roxmltree::Document;
use zip::ZipArchive;

use crate::hydrus_constants::{
    IMAGE_FILE_EXTS, Mime, MimeLookup, OPEN_DOCUMENT_ZIPS, VIDEO_FILE_EXTS,
};
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_text;

const MACOSX_PREFIX: &str = "__MACOSX/";
const CBZ_WEIRD_FILENAMES: &str = "comicinfo.xml,info.txt,thumbs.db,.ds_store";
const CBZ_WEIRD_EXTENSIONS: &str = ".db, .txt, .xml, .nfo, .sfv, .md5, .sha1, .sha256, .sha512";
const CBZ_MAX_WEIRD_FILES_PER_DIRECTORY: f64 = 5.0;
const CBZ_MAX_NUMBERLESS_UNIQUENESS_PERCENTAGE: f64 = 0.2;

fn damaged(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DamagedOrUnusualFile, message)
}

fn no_thumbnail(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::NoThumbnailFile, message)
}

fn zip_error(context: &str, error: impl std::fmt::Display) -> HydrusException {
    damaged(format!("{context}: {error}"))
}

fn open_zip(path: &Path) -> HydrusResult<ZipArchive<File>> {
    let file = File::open(path).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!(
                "Could not open the zip file \"{}\": {error}",
                path.display()
            ),
        )
    })?;

    ZipArchive::new(file).map_err(|error| zip_error("Could not read the zip file", error))
}

/// The names of every entry, including directories, in archive order.
pub fn zip_file_names<R: Read + Seek>(zip_handle: &mut ZipArchive<R>) -> HydrusResult<Vec<String>> {
    let mut names = Vec::with_capacity(zip_handle.len());

    for index in 0..zip_handle.len() {
        let file = zip_handle
            .by_index(index)
            .map_err(|error| zip_error("Could not read a zip entry", error))?;

        names.push(
            file.name()
                .map(|name| name.into_owned())
                .unwrap_or_default(),
        );
    }

    Ok(names)
}

/// `extract_single_file_from_zip( path_to_zip, filename_to_extract, extract_into_file_path )`\n///
/// A missing entry is [`ErrorKind::FileMissing`], matching the `KeyError` the
/// Python implementation lets through.
pub fn extract_single_file_from_zip(
    path_to_zip: impl AsRef<Path>,
    filename_to_extract: &str,
    extract_into_file_path: impl AsRef<Path>,
) -> HydrusResult<()> {
    let mut zip_handle = open_zip(path_to_zip.as_ref())?;

    let mut reader = zip_handle.by_name(filename_to_extract).map_err(|error| {
        if matches!(error, zip::result::ZipError::FileNotFound) {
            HydrusException::new(
                ErrorKind::FileMissing,
                format!("The zip file had no entry called \"{filename_to_extract}\"!"),
            )
        } else {
            zip_error("Could not read the zip entry", error)
        }
    })?;

    let mut bytes = Vec::new();

    reader
        .read_to_end(&mut bytes)
        .map_err(|error| zip_error("Could not read the zip entry", error))?;

    std::fs::write(extract_into_file_path.as_ref(), bytes).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!(
                "Could not write the extracted zip entry to \"{}\": {error}",
                extract_into_file_path.as_ref().display()
            ),
        )
    })
}

/// A lazy handle to one path inside a zip, like Python's `zipfile.Path`.
#[derive(Clone, Debug)]
pub struct ZipPath {
    path_to_zip: PathBuf,
    path_in_zip: String,
}

impl ZipPath {
    pub fn new(path_to_zip: impl Into<PathBuf>, path_in_zip: impl Into<String>) -> Self {
        Self {
            path_to_zip: path_to_zip.into(),
            path_in_zip: path_in_zip.into(),
        }
    }

    /// `zipfile.Path.exists()`
    pub fn exists(&self) -> bool {
        let Ok(mut zip_handle) = open_zip(&self.path_to_zip) else {
            return false;
        };

        zip_handle.by_name(&self.path_in_zip).is_ok()
    }

    /// `zipfile.Path.read_bytes()`
    pub fn read_bytes(&self) -> HydrusResult<Vec<u8>> {
        let mut zip_handle = open_zip(&self.path_to_zip)?;

        let mut reader = zip_handle.by_name(&self.path_in_zip).map_err(|error| {
            if matches!(error, zip::result::ZipError::FileNotFound) {
                HydrusException::new(
                    ErrorKind::FileMissing,
                    format!("The zip file had no entry called \"{}\"!", self.path_in_zip),
                )
            } else {
                zip_error("Could not read the zip entry", error)
            }
        })?;

        let mut bytes = Vec::new();

        reader
            .read_to_end(&mut bytes)
            .map_err(|error| zip_error("Could not read the zip entry", error))?;

        Ok(bytes)
    }

    /// `zipfile.Path.read_text()`
    pub fn read_text(&self) -> HydrusResult<String> {
        let bytes = self.read_bytes()?;

        String::from_utf8(bytes).map_err(|error| {
            damaged(format!(
                "The zip entry \"{}\" was not valid UTF-8: {error}",
                self.path_in_zip
            ))
        })
    }

    pub fn path_in_zip(&self) -> &str {
        &self.path_in_zip
    }
}

/// `get_zip_as_path( path_to_zip, path_in_zip )`
pub fn get_zip_as_path(path_to_zip: impl Into<PathBuf>, path_in_zip: impl Into<String>) -> ZipPath {
    ZipPath::new(path_to_zip, path_in_zip)
}

/// `get_single_file_from_zip_bytes( path_to_zip, path_in_zip )`
pub fn get_single_file_from_zip_bytes(
    path_to_zip: impl AsRef<Path>,
    path_in_zip: &str,
) -> HydrusResult<Vec<u8>> {
    get_zip_as_path(path_to_zip.as_ref().to_path_buf(), path_in_zip).read_bytes()
}

/// `get_cover_page_path( zip_handle )`
pub fn get_cover_page_path<R: Read + Seek>(zip_handle: &mut ZipArchive<R>) -> HydrusResult<String> {
    let mut names = Vec::new();

    for index in 0..zip_handle.len() {
        let file = zip_handle
            .by_index(index)
            .map_err(|error| zip_error("Could not read a zip entry", error))?;

        if file.is_dir() {
            continue;
        }

        let name = file
            .name()
            .map(|name| name.into_owned())
            .unwrap_or_default();

        names.push(name);
    }

    hydrus_text::human_text_sort_strings(&mut names);

    for name in names {
        if name.starts_with(MACOSX_PREFIX) {
            continue;
        }

        let extension = Path::new(&name)
            .extension()
            .map(|extension| format!(".{}", extension.to_string_lossy().to_lowercase()));

        if extension
            .as_deref()
            .is_some_and(|extension| IMAGE_FILE_EXTS.contains(extension))
        {
            return Ok(name);
        }
    }

    Err(no_thumbnail(
        "Sorry, could not find an image file in there!",
    ))
}

fn join_zip_path(directory: &str, filename: &str) -> String {
    if directory.is_empty() {
        filename.to_string()
    } else {
        format!("{directory}/{filename}")
    }
}

/// `get_cover_page_path_from_epub( zip_handle )`
pub fn get_cover_page_path_from_epub<R: Read + Seek>(
    zip_handle: &mut ZipArchive<R>,
) -> HydrusResult<String> {
    let container_bytes = read_entry(zip_handle, "META-INF/container.xml")?;

    let container_text = String::from_utf8(container_bytes).map_err(|error| {
        damaged(format!(
            "The EPUB container.xml was not valid UTF-8: {error}"
        ))
    })?;

    let container = Document::parse(&container_text)
        .map_err(|error| damaged(format!("Bad EPUB XML: {error}")))?;

    let content_opf_path = container
        .descendants()
        .find(|node| node.tag_name().name() == "rootfile")
        .and_then(|node| node.attribute("full-path"))
        .map(str::to_string)
        .ok_or_else(|| no_thumbnail("Sorry, could not find a cover image in the EPUB xml!"))?;

    if !zip_contains(zip_handle, &content_opf_path)? {
        return Err(no_thumbnail(
            "Sorry, could not find the EPUB's content opf!",
        ));
    }

    let content_bytes = read_entry(zip_handle, &content_opf_path)?;

    let content_text = String::from_utf8(content_bytes)
        .map_err(|error| damaged(format!("The EPUB content.opf was not valid UTF-8: {error}")))?;

    let content = Document::parse(&content_text)
        .map_err(|error| damaged(format!("Bad EPUB XML: {error}")))?;

    let content_directory = content_opf_path
        .rsplit_once('/')
        .map(|(directory, _)| directory.to_string())
        .unwrap_or_default();

    let mut cover_item = content.descendants().find(|node| {
        node.tag_name().name() == "item"
            && node
                .attribute("properties")
                .is_some_and(|properties| properties.split_whitespace().any(|p| p == "cover-image"))
    });

    if cover_item.is_none() {
        let cover_id = content
            .descendants()
            .find(|node| {
                node.tag_name().name() == "meta" && node.attribute("name") == Some("cover")
            })
            .and_then(|node| node.attribute("content"));

        if let Some(cover_id) = cover_id {
            cover_item = content.descendants().find(|node| {
                node.tag_name().name() == "item" && node.attribute("id") == Some(cover_id)
            });
        }
    }

    if cover_item.is_none() {
        cover_item = content
            .descendants()
            .find(|node| node.tag_name().name() == "item" && node.attribute("id") == Some("cover"));
    }

    if cover_item.is_none() {
        // Apple's iBook format uses a guide reference instead.
        if let Some(reference) = content.descendants().find(|node| {
            node.tag_name().name() == "reference" && node.attribute("type") == Some("cover")
        }) {
            let href = reference.attribute("href").ok_or_else(|| {
                no_thumbnail("Sorry, could not find a cover image in the EPUB xml!")
            })?;

            let cover_path = join_zip_path(&content_directory, href);

            if !zip_contains(zip_handle, &cover_path)? {
                return Err(no_thumbnail(
                    "Sorry, could not find a cover image in the EPUB xml!",
                ));
            }

            return Ok(cover_path);
        }
    }

    let cover_item = cover_item
        .ok_or_else(|| no_thumbnail("Sorry, could not find a cover image in the EPUB xml!"))?;

    let cover_path = cover_item
        .attribute("href")
        .ok_or_else(|| no_thumbnail("Sorry, could not find a cover image in the EPUB xml!"))?;

    let full_cover_path = join_zip_path(&content_directory, cover_path);

    if !zip_contains(zip_handle, &full_cover_path)? {
        // Some EPUBs point at a missing file; the Python code returns the path
        // anyway and lets the caller's open() fail. Be helpful instead.
        return Err(no_thumbnail(
            "Sorry, could not find a cover image in the EPUB xml!",
        ));
    }

    Ok(full_cover_path)
}

fn read_entry<R: Read + Seek>(zip_handle: &mut ZipArchive<R>, name: &str) -> HydrusResult<Vec<u8>> {
    let mut reader = zip_handle.by_name(name).map_err(|error| {
        if matches!(error, zip::result::ZipError::FileNotFound) {
            no_thumbnail(format!("The zip file had no entry called \"{name}\"!"))
        } else {
            zip_error("Could not read the zip entry", error)
        }
    })?;

    let mut bytes = Vec::new();

    reader
        .read_to_end(&mut bytes)
        .map_err(|error| zip_error("Could not read the zip entry", error))?;

    Ok(bytes)
}

fn zip_contains<R: Read + Seek>(zip_handle: &mut ZipArchive<R>, name: &str) -> HydrusResult<bool> {
    Ok(zip_handle.by_name(name).is_ok())
}

/// `extract_cover_page( path_to_zip, extract_path, mime )`
pub fn extract_cover_page(
    path_to_zip: impl AsRef<Path>,
    extract_path: impl AsRef<Path>,
    mime: Mime,
) -> HydrusResult<()> {
    let mut zip_handle = open_zip(path_to_zip.as_ref())?;

    let path = if mime == Mime::Epub {
        get_cover_page_path_from_epub(&mut zip_handle)?
    } else {
        get_cover_page_path(&mut zip_handle)?
    };

    let mut reader = zip_handle
        .by_name(&path)
        .map_err(|error| zip_error("Could not read the cover image from the zip", error))?;

    let mut bytes = Vec::new();

    reader
        .read_to_end(&mut bytes)
        .map_err(|error| zip_error("Could not read the cover image from the zip", error))?;

    std::fs::write(extract_path.as_ref(), bytes).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!(
                "Could not write the cover image to \"{}\": {error}",
                extract_path.as_ref().display()
            ),
        )
    })
}

/// `is_encrypted_zip( path_to_zip )`
pub fn is_encrypted_zip(path_to_zip: impl AsRef<Path>) -> HydrusResult<bool> {
    let mut zip_handle = open_zip(path_to_zip.as_ref())?;

    for index in 0..zip_handle.len() {
        let file = zip_handle
            .by_index(index)
            .map_err(|error| zip_error("Could not read a zip entry", error))?;

        if file.encrypted() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn filename_has_extension(filename: &str, extensions: &HashSet<&'static str>) -> bool {
    let Some((_, extension)) = filename.rsplit_once('.') else {
        return false;
    };

    let extension = format!(".{}", extension.to_lowercase());

    extensions.contains(extension.as_str())
}

/// `filename_has_image_ext( filename )`
pub fn filename_has_image_ext(filename: &str) -> bool {
    filename_has_extension(filename, &IMAGE_FILE_EXTS)
}

/// `filename_has_video_ext( filename )`
pub fn filename_has_video_ext(filename: &str) -> bool {
    filename_has_extension(filename, &VIDEO_FILE_EXTS)
}

fn digits_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();

    REGEX.get_or_init(|| Regex::new(r"\d").unwrap())
}

/// `zip_looks_like_cbz( path_to_zip )`
pub fn zip_looks_like_cbz(path_to_zip: impl AsRef<Path>) -> bool {
    let path_to_zip = path_to_zip.as_ref();

    let Ok(mut zip_handle) = open_zip(path_to_zip) else {
        return false;
    };

    let weird_filenames: HashSet<&str> = CBZ_WEIRD_FILENAMES.split(',').collect();
    let weird_extensions: HashSet<&str> = CBZ_WEIRD_EXTENSIONS.split(',').map(str::trim).collect();

    let mut directories_to_image_filenames: HashMap<String, HashSet<String>> = HashMap::new();
    let mut directories_with_stuff_in: HashSet<String> = HashSet::new();

    let mut num_weird_files = 0usize;
    let mut num_images = 0usize;
    let mut num_images_with_numbers = 0usize;

    let names = match zip_file_names(&mut zip_handle) {
        Ok(names) => names,
        Err(_) => return false,
    };

    for name in names {
        if name.starts_with(MACOSX_PREFIX) {
            continue;
        }

        if name.ends_with('/') {
            continue;
        }

        let (directory, filename) = name
            .rsplit_once('/')
            .map(|(directory, filename)| (directory.to_string(), filename.to_string()))
            .unwrap_or_else(|| (String::new(), name.clone()));

        directories_with_stuff_in.insert(directory.clone());

        let filename_lower = filename.to_lowercase();

        if weird_filenames.contains(filename_lower.as_str()) {
            continue;
        }

        if filename_has_image_ext(&filename_lower) {
            num_images += 1;

            if digits_regex().is_match(&filename_lower) {
                num_images_with_numbers += 1;
            }

            directories_to_image_filenames
                .entry(directory)
                .or_default()
                .insert(filename_lower);
        } else {
            if let Some((_, extension)) = filename_lower.rsplit_once('.') {
                let extension = format!(".{extension}");

                if weird_extensions.contains(extension.as_str()) {
                    num_weird_files += 1;
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
    }

    if num_images_with_numbers <= 1 {
        return false;
    }

    let Ok(cover_page_path) = get_cover_page_path(&mut zip_handle) else {
        return false;
    };

    let Ok(mut cover_reader) = zip_handle.by_name(&cover_page_path) else {
        return false;
    };

    let mut one_byte = [0u8; 1];

    if cover_reader.read(&mut one_byte).is_err() {
        return false;
    }

    if !directories_to_image_filenames.is_empty() {
        let mut percentages = Vec::with_capacity(directories_to_image_filenames.len());

        for image_filenames in directories_to_image_filenames.values() {
            let unique_numberless_filenames: HashSet<String> = image_filenames
                .iter()
                .map(|filename| digits_regex().replace_all(filename, "").into_owned())
                .collect();

            let magical_uniqueness_percentage =
                (unique_numberless_filenames.len() as f64 - 1.0) / image_filenames.len() as f64;

            percentages.push(magical_uniqueness_percentage);
        }

        let average_uniqueness = percentages.iter().sum::<f64>() / percentages.len() as f64;

        if average_uniqueness > CBZ_MAX_NUMBERLESS_UNIQUENESS_PERCENTAGE {
            return false;
        }
    }

    if (num_weird_files as f64) * (directories_with_stuff_in.len() as f64)
        > CBZ_MAX_WEIRD_FILES_PER_DIRECTORY
    {
        return false;
    }

    if num_images * directories_with_stuff_in.len() < 1 {
        return false;
    }

    true
}

/// `mime_from_open_document( path )`
pub fn mime_from_open_document(path: impl AsRef<Path>) -> Option<Mime> {
    let mimetype_data = get_zip_as_path(path.as_ref().to_path_buf(), "mimetype")
        .read_text()
        .ok()?;

    let filetype = Mime::from_mimetype(&mimetype_data)?;

    match filetype {
        MimeLookup::One(mime) if OPEN_DOCUMENT_ZIPS.contains(&mime) => Some(mime),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buffer = Cursor::new(Vec::new());

        {
            let mut writer = ZipWriter::new(&mut buffer);
            let options = SimpleFileOptions::default();

            for (name, data) in entries {
                writer.start_file(*name, options).unwrap();
                writer.write_all(data).unwrap();
            }

            writer.finish().unwrap();
        }

        buffer.into_inner()
    }

    fn write_temp_zip(name: &str, entries: &[(&str, &[u8])]) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "hydrus_archive_{}_{}_{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        std::fs::write(&path, build_zip(entries)).unwrap();

        path
    }

    #[test]
    fn cover_page_is_the_first_image_in_human_order() {
        let data = build_zip(&[
            ("page10.jpg", b"10"),
            ("page2.jpg", b"2"),
            ("notes.txt", b"hello"),
        ]);

        let mut archive = ZipArchive::new(Cursor::new(data)).unwrap();
        let cover = get_cover_page_path(&mut archive).unwrap();

        assert_eq!(cover, "page2.jpg");
    }

    #[test]
    fn cover_page_skips_macosx_metadata() {
        let data = build_zip(&[("__MACOSX/._page1.jpg", b"junk"), ("page1.jpg", b"1")]);

        let mut archive = ZipArchive::new(Cursor::new(data)).unwrap();
        let cover = get_cover_page_path(&mut archive).unwrap();

        assert_eq!(cover, "page1.jpg");
    }

    #[test]
    fn no_image_is_a_no_thumbnail_error() {
        let data = build_zip(&[("notes.txt", b"hello")]);

        let mut archive = ZipArchive::new(Cursor::new(data)).unwrap();
        let error = get_cover_page_path(&mut archive).unwrap_err();

        assert!(error.is_a(ErrorKind::NoThumbnailFile));
    }

    #[test]
    fn epub3_cover_image_property_is_understood() {
        let container = r#"<?xml version="1.0"?>
<container xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf"/></rootfiles>
</container>"#;

        let opf = r#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf">
  <manifest>
    <item id="cover" href="images/cover.jpg" properties="cover-image"/>
  </manifest>
</package>"#;

        let data = build_zip(&[
            ("META-INF/container.xml", container.as_bytes()),
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/images/cover.jpg", b"jpg"),
        ]);

        let mut archive = ZipArchive::new(Cursor::new(data)).unwrap();
        let cover = get_cover_page_path_from_epub(&mut archive).unwrap();

        assert_eq!(cover, "OEBPS/images/cover.jpg");
    }

    #[test]
    fn epub2_meta_cover_is_understood() {
        let container = r#"<?xml version="1.0"?>
<container xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="content.opf"/></rootfiles>
</container>"#;

        let opf = r#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf">
  <metadata><meta name="cover" content="my-cover"/></metadata>
  <manifest>
    <item id="my-cover" href="cover.png"/>
  </manifest>
</package>"#;

        let data = build_zip(&[
            ("META-INF/container.xml", container.as_bytes()),
            ("content.opf", opf.as_bytes()),
            ("cover.png", b"png"),
        ]);

        let mut archive = ZipArchive::new(Cursor::new(data)).unwrap();
        let cover = get_cover_page_path_from_epub(&mut archive).unwrap();

        assert_eq!(cover, "cover.png");
    }

    #[test]
    fn cbz_heuristic_accepts_chapters_of_numbered_images() {
        let path = write_temp_zip(
            "cbz",
            &[
                ("Chapter 1/001.jpg", b"1"),
                ("Chapter 1/002.jpg", b"2"),
                ("Chapter 2/001.jpg", b"3"),
                ("Chapter 2/002.jpg", b"4"),
            ],
        );

        assert!(zip_looks_like_cbz(&path));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn cbz_heuristic_rejects_a_single_loose_file() {
        let path = write_temp_zip("not_cbz", &[("image.jpg", b"1")]);

        assert!(!zip_looks_like_cbz(&path));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn open_document_mimetype_is_resolved() {
        let path = write_temp_zip("kra", &[("mimetype", b"application/x-krita")]);

        assert_eq!(mime_from_open_document(&path), Some(Mime::Krita));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn extracting_a_single_file_writes_its_bytes() {
        let path = write_temp_zip("extract", &[("Thumbnail.png", b"png-bytes")]);

        let output = std::env::temp_dir().join(format!(
            "hydrus_archive_extracted_{}.png",
            std::process::id()
        ));

        extract_single_file_from_zip(&path, "Thumbnail.png", &output).unwrap();

        assert_eq!(std::fs::read(&output).unwrap(), b"png-bytes");

        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(output).unwrap();
    }

    #[test]
    fn a_missing_zip_entry_is_file_missing() {
        let path = write_temp_zip("missing", &[("a.txt", b"a")]);

        let error = get_zip_as_path(path.clone(), "nope.txt")
            .read_bytes()
            .unwrap_err();

        assert!(error.is_a(ErrorKind::FileMissing));

        std::fs::remove_file(path).unwrap();
    }
}
