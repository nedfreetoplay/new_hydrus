//! Rust port of `hydrus/core/files/HydrusUgoiraHandling.py`.
//!
//! Ugoira files are ZIPs of numbered image frames, optionally accompanied by an
//! `animation.json` containing per-frame delays. The Python module caches the
//! parsed JSON in a small LRU; the port uses a process-wide map behind a mutex,
//! which has the same observable effect for normal import workloads.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use image::DynamicImage;
use serde_json::Value;
use zip::ZipArchive;

use crate::files::hydrus_archive_handling;
use crate::hydrus_constants::IMAGE_FILE_EXTS;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::files::images::hydrus_image_handling;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UgoiraFrame {
    pub file: String,
    pub delay: i64,
}

fn limited_support(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::LimitedSupportFile, message)
}

fn damaged(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DamagedOrUnusualFile, message)
}

fn frame_cache() -> &'static Mutex<HashMap<PathBuf, Option<Vec<UgoiraFrame>>>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<Vec<UgoiraFrame>>>>> = OnceLock::new();

    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_frame_cache() -> MutexGuard<'static, HashMap<PathBuf, Option<Vec<UgoiraFrame>>>> {
    frame_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn parse_frame_data(value: &Value) -> Option<Vec<UgoiraFrame>> {
    let frames_value = match value {
        Value::Array(items) => items,
        Value::Object(entries) => entries.get("frames")?.as_array()?,
        _ => return None,
    };

    let mut frames = Vec::with_capacity(frames_value.len());

    for frame in frames_value {
        let object = frame.as_object()?;

        let file = object.get("file")?.as_str()?.to_string();
        let delay = object.get("delay")?.as_i64()?;

        frames.push(UgoiraFrame { file, delay });
    }

    Some(frames)
}

/// `get_frame_paths_from_ugoira_zip( path )`
pub fn get_frame_paths_from_ugoira_zip(path: impl AsRef<Path>) -> HydrusResult<Vec<String>> {
    let path = path.as_ref();

    let file = std::fs::File::open(path).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!("Could not open the Ugoira file \"{}\": {error}", path.display()),
        )
    })?;

    let mut zip_handle = ZipArchive::new(file).map_err(|error| {
        damaged(format!(
            "Could not read the Ugoira zip \"{}\": {error}",
            path.display()
        ))
    })?;

    let mut paths = Vec::new();

    for index in 0..zip_handle.len() {
        let entry = zip_handle.by_index(index).map_err(|error| {
            damaged(format!(
                "Could not read a Ugoira zip entry from \"{}\": {error}",
                path.display()
            ))
        })?;

        if entry.is_dir() {
            continue;
        }

        let name = entry
            .name()
            .map(|name| name.to_string())
            .unwrap_or_default();

        if hydrus_archive_handling::filename_has_image_ext(&name) {
            paths.push(name);
        }
    }

    if paths.is_empty() {
        return Err(damaged(
            "This Ugoira seems to be empty! It has probably been corrupted!",
        ));
    }

    paths.sort();

    Ok(paths)
}

/// `get_ugoira_json( path )`
pub fn get_ugoira_json(path: impl AsRef<Path>) -> HydrusResult<Value> {
    let path = path.as_ref();

    let json_file = hydrus_archive_handling::get_zip_as_path(
        path.to_path_buf(),
        "animation.json",
    );

    if !json_file.exists() {
        return Err(limited_support("Zip file has no animation.json!"));
    }

    let text = json_file.read_text().map_err(|_| {
        limited_support("Zip file has no animation.json or it cannot be parsed")
    })?;

    serde_json::from_str(&text).map_err(|_| {
        limited_support("Zip file has no animation.json or it cannot be parsed")
    })
}

/// `get_ugoira_frame_data_json( path )`
pub fn get_ugoira_frame_data_json(path: impl AsRef<Path>) -> Option<Vec<UgoiraFrame>> {
    let path = path.as_ref().to_path_buf();

    if let Some(cached) = lock_frame_cache().get(&path).cloned() {
        return cached;
    }

    let frames = get_ugoira_json(&path)
        .ok()
        .and_then(|value| parse_frame_data(&value));

    lock_frame_cache().insert(path, frames.clone());

    frames
}

/// `get_ugoira_properties_from_json( path )`
pub fn get_ugoira_properties_from_json(
    path: impl AsRef<Path>,
) -> HydrusResult<((u32, u32), i64, u32)> {
    let path = path.as_ref();

    let frame_data = get_ugoira_frame_data_json(path).ok_or_else(|| {
        limited_support("Zip file has no animation.json or it cannot be parsed")
    })?;

    let duration_ms = frame_data.iter().map(|frame| frame.delay).sum::<i64>();
    let num_frames = frame_data.len() as u32;

    let first_frame = get_ugoira_frame_pil(path, 0)?;
    let (width, height) = (first_frame.width(), first_frame.height());

    Ok(((width, height), duration_ms, num_frames))
}

/// `get_ugoira_properties( path_to_zip )`
pub fn get_ugoira_properties(
    path: impl AsRef<Path>,
) -> HydrusResult<((u32, u32), Option<i64>, Option<u32>)> {
    let path = path.as_ref();

    if let Ok(((width, height), duration_ms, num_frames)) =
        get_ugoira_properties_from_json(path)
    {
        return Ok(((width, height), Some(duration_ms), Some(num_frames)));
    }

    let (width, height) = match get_ugoira_frame_pil(path, 0) {
        Ok(frame) => (frame.width(), frame.height()),
        Err(_) => (100, 100),
    };

    let num_frames = get_frame_paths_from_ugoira_zip(path)
        .ok()
        .map(|paths| paths.len() as u32);

    Ok(((width, height), None, num_frames))
}

/// `zip_looks_like_ugoira( path_to_zip )`
pub fn zip_looks_like_ugoira(path_to_zip: impl AsRef<Path>) -> bool {
    let path_to_zip = path_to_zip.as_ref();

    if let Some(frames) = get_ugoira_frame_data_json(path_to_zip) {
        if !frames.is_empty() {
            return true;
        }
    }

    let Ok(file) = std::fs::File::open(path_to_zip) else {
        return false;
    };

    let Ok(mut zip_handle) = ZipArchive::new(file) else {
        return false;
    };

    if (0..zip_handle.len()).any(|index| {
        zip_handle
            .by_index(index)
            .map(|entry| entry.is_dir())
            .unwrap_or(false)
    }) {
        return false;
    }

    let mut our_image_ext: Option<String> = None;
    let mut image_number_strings = Vec::new();

    let mut filenames = Vec::with_capacity(zip_handle.len());

    for index in 0..zip_handle.len() {
        let Ok(entry) = zip_handle.by_index(index) else {
            return false;
        };

        let Ok(name) = entry.name() else {
            return false;
        };

        filenames.push(name.to_string());
    }

    for filename in filenames {
        if !filename.contains('.') {
            return false;
        }

        let mut pieces = filename.rsplitn(2, '.');

        let Some(extension) = pieces.next() else {
            return false;
        };
        let Some(number) = pieces.next() else {
            return false;
        };

        let ext = format!(".{extension}");

        if ext == ".js" || ext == ".json" {
            continue;
        }

        if !IMAGE_FILE_EXTS.contains(ext.as_str()) {
            return false;
        }

        match &our_image_ext {
            None => our_image_ext = Some(ext),
            Some(expected) if *expected != ext => return false,
            Some(_) => {}
        }

        image_number_strings.push(number.to_string());
    }

    if image_number_strings.len() <= 1 {
        return false;
    }

    for (expected_image_number, image_number_string) in
        image_number_strings.iter().enumerate()
    {
        if *image_number_string != format!("{expected_image_number:06}") {
            return false;
        }
    }

    let cover_path = match hydrus_archive_handling::get_cover_page_path(&mut zip_handle) {
        Ok(path) => path,
        Err(_) => return false,
    };

    if hydrus_archive_handling::get_zip_as_path(path_to_zip.to_path_buf(), cover_path)
        .read_bytes()
        .is_err()
    {
        return false;
    }

    true
}

/// `get_frame_paths_ugoira( path )`
pub fn get_frame_paths_ugoira(path: impl AsRef<Path>) -> HydrusResult<Vec<String>> {
    let path = path.as_ref();

    if let Some(frames) = get_ugoira_frame_data_json(path) {
        return Ok(frames.into_iter().map(|frame| frame.file).collect());
    }

    get_frame_paths_from_ugoira_zip(path)
}

/// `get_ugoira_frame_pil( path, frameIndex )`
pub fn get_ugoira_frame_pil(
    path: impl AsRef<Path>,
    frame_index: usize,
) -> HydrusResult<DynamicImage> {
    let path = path.as_ref();

    let frame_paths = get_frame_paths_ugoira(path)?;

    let frame_name = frame_paths.get(frame_index).ok_or_else(|| {
        damaged(format!(
            "Ugoira frame {frame_index} is missing from \"{}\"!",
            path.display()
        ))
    })?;

    let bytes =
        hydrus_archive_handling::get_zip_as_path(path.to_path_buf(), frame_name.clone())
            .read_bytes()?;

    hydrus_image_handling::decode_image_bytes(&bytes)
}

/// `generate_thumbnail_numpy_from_ugoira_path( path, target_resolution, frame_index )`
pub fn generate_thumbnail_numpy_from_ugoira_path(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
    frame_index: usize,
) -> HydrusResult<DynamicImage> {
    let frame = get_ugoira_frame_pil(path, frame_index)?;

    Ok(hydrus_image_handling::resize_image(
        &frame,
        target_resolution,
        None,
    ))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    use super::*;

    fn unique_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "hydrus_ugoira_{}_{}_{}.zip",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::new_rgb8(width, height);
        let mut bytes = Vec::new();

        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();

        bytes
    }

    fn write_ugoira(label: &str, with_json: bool) -> PathBuf {
        let path = unique_path(label);

        let file = std::fs::File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);

        zip.start_file("000000.png", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&png_bytes(4, 3)).unwrap();

        zip.start_file("000001.png", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&png_bytes(4, 3)).unwrap();

        if with_json {
            zip.start_file("animation.json", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(
                br#"[{"file":"000000.png","delay":100},{"file":"000001.png","delay":250}]"#,
            )
            .unwrap();
        }

        zip.finish().unwrap();

        path
    }

    #[test]
    fn numbered_image_zip_is_recognised() {
        let path = write_ugoira("plain", false);

        assert!(zip_looks_like_ugoira(&path));
        assert_eq!(
            get_frame_paths_from_ugoira_zip(&path).unwrap(),
            vec!["000000.png".to_string(), "000001.png".to_string()]
        );

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn json_delays_give_duration_and_frame_count() {
        let path = write_ugoira("json", true);

        assert!(zip_looks_like_ugoira(&path));

        let ((width, height), duration_ms, num_frames) =
            get_ugoira_properties(&path).unwrap();

        assert_eq!((width, height), (4, 3));
        assert_eq!(duration_ms, Some(350));
        assert_eq!(num_frames, Some(2));

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn object_json_with_frames_key_works_too() {
        let value: Value = serde_json::from_str(
            r#"{"frames":[{"file":"a.png","delay":50}]}"#,
        )
        .unwrap();

        let frames = parse_frame_data(&value).unwrap();

        assert_eq!(frames[0].file, "a.png");
        assert_eq!(frames[0].delay, 50);
    }

    #[test]
    fn a_single_frame_is_not_an_ugoira() {
        let path = unique_path("single");
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);

        zip.start_file("000000.png", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&png_bytes(2, 2)).unwrap();
        zip.finish().unwrap();

        assert!(!zip_looks_like_ugoira(&path));

        std::fs::remove_file(path).ok();
    }
}
