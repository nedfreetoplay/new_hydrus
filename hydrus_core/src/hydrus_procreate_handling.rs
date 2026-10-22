//! Rust port of `hydrus/core/files/HydrusProcreateHandling.py`.
//!
//! Procreate documents are zips. `Document.archive` is an NSKeyedArchiver
//! plist that names the `SilicaDocument` class and stores the canvas size as a
//! referenced string such as `{2894, 4093}`.

use std::io::Cursor;
use std::path::Path;

use plist::Value;

use crate::hydrus_archive_handling;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

pub const PROCREATE_THUMBNAIL_FILE_PATH: &str = "QuickLook/Thumbnail.png";
pub const PROCREATE_DOCUMENT_ARCHIVE: &str = "Document.archive";
pub const PROCREATE_PROJECT_KEY: usize = 1;

/// `extract_zipped_thumbnail_to_path( path_to_zip, temp_path_file )`
pub fn extract_zipped_thumbnail_to_path(
    path_to_zip: impl AsRef<Path>,
    temp_path_file: impl AsRef<Path>,
) -> HydrusResult<()> {
    hydrus_archive_handling::extract_single_file_from_zip(
        path_to_zip,
        PROCREATE_THUMBNAIL_FILE_PATH,
        temp_path_file,
    )
    .map_err(|error| {
        if error.is_a(ErrorKind::FileMissing) {
            HydrusException::new(
                ErrorKind::NoThumbnailFile,
                "This procreate file had no thumbnail file!",
            )
        } else {
            error
        }
    })
}

/// `get_procreate_plist( path )`
pub fn get_procreate_plist(path: impl AsRef<Path>) -> HydrusResult<Value> {
    let plist_path = hydrus_archive_handling::get_zip_as_path(
        path.as_ref().to_path_buf(),
        PROCREATE_DOCUMENT_ARCHIVE,
    );

    if !plist_path.exists() {
        return Err(HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Procreate file has no plist!",
        ));
    }

    let bytes = plist_path.read_bytes()?;

    Value::from_reader(Cursor::new(bytes)).map_err(|error| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            format!("Could not parse the Procreate plist: {error}"),
        )
    })
}

fn objects_of(document: &Value) -> Option<&Vec<Value>> {
    match document {
        Value::Dictionary(dictionary) => dictionary.get("$objects").and_then(Value::as_array),
        _ => None,
    }
}

fn pointer_of(value: &Value) -> Option<usize> {
    match value {
        Value::Uid(uid) => Some(uid.get() as usize),
        Value::Integer(integer) => integer.as_unsigned().map(|value| value as usize),
        Value::Dictionary(dictionary) => dictionary
            .get("CF$UID")
            .and_then(pointer_of),
        _ => None,
    }
}

fn integer_of(value: &Value) -> Option<i64> {
    match value {
        Value::Uid(uid) => Some(uid.get() as i64),
        Value::Integer(integer) => integer.as_signed(),
        Value::Dictionary(dictionary) => dictionary.get("CF$UID").and_then(pointer_of).map(|value| value as i64),
        _ => None,
    }
}

/// `zip_looks_like_procreate( path )`
pub fn zip_looks_like_procreate(path: impl AsRef<Path>) -> bool {
    (|| -> Option<bool> {
        let document = get_procreate_plist(path).ok()?;
        let objects = objects_of(&document)?;
        let project = objects.get(PROCREATE_PROJECT_KEY)?;
        let project = project.as_dictionary()?;
        let class_pointer = pointer_of(project.get("$class")?)?;
        let class_object = objects.get(class_pointer)?.as_dictionary()?;
        let class_name = class_object.get("$classname")?.as_string()?;

        Some(class_name == "SilicaDocument")
    })()
    .unwrap_or(false)
}

/// `get_procreate_resolution( path )`
pub fn get_procreate_resolution(path: impl AsRef<Path>) -> HydrusResult<(u32, u32)> {
    let result = (|| -> Option<(u32, u32)> {
        let document = get_procreate_plist(path).ok()?;
        let objects = objects_of(&document)?;
        let project = objects.get(PROCREATE_PROJECT_KEY)?.as_dictionary()?;

        let dimension_pointer = pointer_of(project.get("size")?)?;
        let size_string = objects.get(dimension_pointer)?.as_string()?;

        let size_string = size_string
            .trim_matches(|character| character == '{' || character == '}')
            .to_string();

        let mut pieces = size_string.split(", ");

        let first = pieces.next()?.trim().parse::<u32>().ok()?;
        let second = pieces.next()?.trim().parse::<u32>().ok()?;

        let orientation = integer_of(project.get("orientation")?)?;

        if orientation == 3 || orientation == 4 {
            Some((second, first))
        } else {
            Some((first, second))
        }
    })();

    result.ok_or_else(|| HydrusException::new(ErrorKind::NoResolutionFile, ""))
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
            "hydrus_procreate_{}_{}_{}.procreate",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn plist_document(orientation: i64) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>$objects</key>
  <array>
    <string>$null</string>
    <dict>
      <key>$class</key><integer>2</integer>
      <key>size</key><dict><key>CF$UID</key><integer>3</integer></dict>
      <key>orientation</key><integer>{orientation}</integer>
    </dict>
    <dict><key>$classname</key><string>SilicaDocument</string></dict>
    <string>{{2894, 4093}}</string>
  </array>
</dict>
</plist>"#
        )
    }

    fn build_procreate(label: &str, document: Option<&str>, thumbnail: Option<&[u8]>) -> PathBuf {
        let path = unique_path(label);

        let file = std::fs::File::create(&path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        if let Some(document) = document {
            writer.start_file(PROCREATE_DOCUMENT_ARCHIVE, options).unwrap();
            writer.write_all(document.as_bytes()).unwrap();
        }

        if let Some(thumbnail) = thumbnail {
            writer.start_file(PROCREATE_THUMBNAIL_FILE_PATH, options).unwrap();
            writer.write_all(thumbnail).unwrap();
        }

        writer.finish().unwrap();

        path
    }

    #[test]
    fn silica_documents_are_recognised() {
        let document = plist_document(0);
        let path = build_procreate("recognise", Some(&document), None);

        assert!(zip_looks_like_procreate(&path));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn resolution_swaps_for_rotated_orientations() {
        let document = plist_document(3);
        let path = build_procreate("rotated", Some(&document), None);

        assert_eq!(get_procreate_resolution(&path).unwrap(), (4093, 2894));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn resolution_is_unswapped_for_normal_orientations() {
        let document = plist_document(1);
        let path = build_procreate("normal", Some(&document), None);

        assert_eq!(get_procreate_resolution(&path).unwrap(), (2894, 4093));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_missing_plist_is_damaged() {
        let path = build_procreate("no_plist", None, None);

        let error = get_procreate_plist(&path).unwrap_err();
        assert!(error.is_a(ErrorKind::DamagedOrUnusualFile));
        assert!(!zip_looks_like_procreate(&path));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_missing_thumbnail_is_a_no_thumbnail_error() {
        let document = plist_document(0);
        let path = build_procreate("no_thumb", Some(&document), None);

        let output = unique_path("out");

        let error = extract_zipped_thumbnail_to_path(&path, &output).unwrap_err();

        assert!(error.is_a(ErrorKind::NoThumbnailFile));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_present_thumbnail_is_written() {
        let document = plist_document(0);
        let path = build_procreate("thumb", Some(&document), Some(b"png-bytes"));

        let output = unique_path("out");

        extract_zipped_thumbnail_to_path(&path, &output).unwrap();

        assert_eq!(std::fs::read(&output).unwrap(), b"png-bytes");

        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(output).unwrap();
    }
}
