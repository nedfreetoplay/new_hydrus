//! Surplus files in the media storage, from `_ClearOrphanFiles` in
//! `ClientGUI.py` and `ClearOrphans` in `ClientFilesManager`.
//!
//! Every media file and thumbnail is named after the hash of the file it
//! belongs to. Anything with a hash-shaped name that the database does not
//! know about is an orphan: it is safe to delete or move away. Files with any
//! other name (sidecars, temporary files) are left alone.

use std::path::{Component, Path, PathBuf};

use serde_json::{json, Value};

use crate::files::ClientFilesManager;

use super::{render, AppState, UiCommand};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrphanFile {
    pub path: PathBuf,
    pub hash: String,
    pub thumbnail: bool,
    pub size: u64,
}

impl OrphanFile {
    fn to_json(&self) -> Value {
        json!({
            "path": self.path.to_string_lossy(),
            "hash": self.hash,
            "thumbnail": self.thumbnail,
            "size": self.size,
            "size_display": human_bytes(self.size),
            "kind": if self.thumbnail { "thumbnail" } else { "file" },
        })
    }
}

/// Lists every file under the configured storage locations whose name is a
/// hash the database does not contain.
pub fn scan(state: &AppState) -> Result<Vec<OrphanFile>, String> {
    let manager = state.controller.files_manager();
    let db = state.controller.db().db();
    let mut orphans = Vec::new();

    for base in manager.locations() {
        if !base.path.exists() {
            continue;
        }

        collect_orphans(db, &base.path, 0, &mut orphans)?;
    }

    orphans.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(orphans)
}

fn collect_orphans(
    db: &hydrus_core::hydrus_db_base::DBBase,
    directory: &Path,
    depth: usize,
    orphans: &mut Vec<OrphanFile>,
) -> Result<(), String> {
    if depth > 8 {
        return Ok(());
    }

    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            collect_orphans(db, &path, depth + 1, orphans)?;
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some((hash, thumbnail)) = parse_stored_name(name) else {
            continue;
        };

        let bytes = match decode_hex(&hash) {
            Some(bytes) => bytes,
            None => continue,
        };

        let known = crate::db::master::has_hash(db, &bytes).unwrap_or(true);

        if known {
            continue;
        }

        let size = entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);

        orphans.push(OrphanFile {
            path,
            hash,
            thumbnail,
            size,
        });
    }

    Ok(())
}

/// Recognises `<64 hex>.ext` and `<64 hex>.thumbnail` file names.
fn parse_stored_name(name: &str) -> Option<(String, bool)> {
    let thumbnail = name.ends_with(".thumbnail");
    let stem = match name.split_once('.') {
        Some((stem, extension)) if !thumbnail || extension == "thumbnail" => stem,
        _ => name,
    };

    if stem.len() != 64 || !stem.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }

    let lower = stem.to_ascii_lowercase();

    Some((lower, thumbnail))
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

pub fn dialog_json(state: &AppState) -> String {
    match scan(state) {
        Ok(orphans) => orphans_json(&orphans),
        Err(error) => json!({
            "orphans": [],
            "status": format!("could not scan the file storage: {error}"),
            "files": 0,
            "thumbnails": 0,
            "bytes": 0,
            "bytes_display": "0 B",
        })
        .to_string(),
    }
}

fn orphans_json(orphans: &[OrphanFile]) -> String {
    let entries: Vec<Value> = orphans.iter().map(OrphanFile::to_json).collect();
    let thumbnails = orphans.iter().filter(|orphan| orphan.thumbnail).count();
    let files = orphans.len() - thumbnails;
    let bytes: u64 = orphans.iter().map(|orphan| orphan.size).sum();

    json!({
        "orphans": entries,
        "status": match orphans.len() {
            0 => "no orphan files found".to_string(),
            1 => "1 orphan file".to_string(),
            count => format!("{count} orphan files"),
        },
        "files": files,
        "thumbnails": thumbnails,
        "bytes": bytes,
        "bytes_display": human_bytes(bytes),
    })
    .to_string()
}

pub fn open(state: &AppState) -> Vec<UiCommand> {
    vec![UiCommand::OrphanFilesDialog(dialog_json(state))]
}

pub fn delete(state: &AppState, payload: &str) -> Vec<UiCommand> {
    let paths = match parse_paths(payload) {
        Ok(paths) => paths,
        Err(error) => return notify(state, error),
    };

    let manager = state.controller.files_manager();
    let db = state.controller.db().db();
    let mut removed = 0usize;
    let mut skipped = 0usize;

    for path in &paths {
        match verify_orphan(&manager, db, path) {
            Ok(true) => match crate::files::delete_file(path) {
                Ok(()) => removed += 1,
                Err(_) => skipped += 1,
            },
            _ => skipped += 1,
        }
    }

    let mut message = format!("deleted {removed} orphan file(s)");

    if skipped > 0 {
        message.push_str(&format!("; {skipped} path(s) were skipped"));
    }

    updated(state, message)
}

pub fn move_to(state: &AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return notify(state, "invalid orphan move payload");
    };
    let target = value
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    if target.is_empty() {
        return notify(state, "choose a directory to move the orphans to");
    }

    let paths = match parse_paths(payload) {
        Ok(paths) => paths,
        Err(error) => return notify(state, error),
    };

    let target = PathBuf::from(target);

    if target
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return notify(state, "the target directory is not valid");
    }

    if let Err(error) = std::fs::create_dir_all(&target) {
        return notify(
            state,
            format!("could not create the target directory: {error}"),
        );
    }

    let manager = state.controller.files_manager();
    let db = state.controller.db().db();
    let mut moved = 0usize;
    let mut skipped = 0usize;

    for path in &paths {
        if !matches!(verify_orphan(&manager, db, path), Ok(true)) {
            skipped += 1;
            continue;
        }

        let thumbnail = path
            .extension()
            .is_some_and(|extension| extension == "thumbnail");
        let folder = if thumbnail {
            target.join("thumbnails")
        } else {
            target.join("files")
        };

        if std::fs::create_dir_all(&folder).is_err() {
            skipped += 1;
            continue;
        }

        let destination = unique_destination(&folder, path);

        if std::fs::rename(path, &destination).is_ok()
            || (std::fs::copy(path, &destination).is_ok() && std::fs::remove_file(path).is_ok())
        {
            moved += 1;
        } else {
            skipped += 1;
        }
    }

    let mut message = format!("moved {moved} orphan file(s) to {}", target.display());

    if skipped > 0 {
        message.push_str(&format!("; {skipped} path(s) were skipped"));
    }

    updated(state, message)
}

/// Re-checks at action time that the path is inside the storage and still has
/// no database record, so a stale dialog cannot delete a file that has since
/// become live.
fn verify_orphan(
    manager: &ClientFilesManager,
    db: &hydrus_core::hydrus_db_base::DBBase,
    path: &Path,
) -> Result<bool, String> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Ok(false);
    }

    let inside = manager
        .locations()
        .iter()
        .any(|location| path.starts_with(&location.path));

    if !inside || !path.is_file() {
        return Ok(false);
    }

    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return Ok(false);
    };
    let Some((hash, _)) = parse_stored_name(name) else {
        return Ok(false);
    };
    let Some(bytes) = decode_hex(&hash) else {
        return Ok(false);
    };

    crate::db::master::has_hash(db, &bytes)
        .map(|known| !known)
        .map_err(|error| error.to_string())
}

fn unique_destination(folder: &Path, source: &Path) -> PathBuf {
    let Some(name) = source.file_name() else {
        return folder.join("orphan");
    };
    let mut candidate = folder.join(name);

    if !candidate.exists() {
        return candidate;
    }

    let name = name.to_string_lossy().to_string();
    let (stem, extension) = match name.split_once('.') {
        Some((stem, extension)) => (stem.to_string(), format!(".{extension}")),
        None => (name, String::new()),
    };

    for index in 1..10_000 {
        candidate = folder.join(format!("{stem}_{index}{extension}"));

        if !candidate.exists() {
            return candidate;
        }
    }

    folder.join(format!("{stem}_{}", std::process::id()))
}

fn parse_paths(payload: &str) -> Result<Vec<PathBuf>, String> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return Err("invalid orphan file payload".to_string());
    };

    let Some(paths) = value.get("paths").and_then(Value::as_array) else {
        return Err("choose at least one orphan file".to_string());
    };

    let paths: Vec<PathBuf> = paths
        .iter()
        .filter_map(Value::as_str)
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .collect();

    if paths.is_empty() {
        return Err("choose at least one orphan file".to_string());
    }

    Ok(paths)
}

fn updated(state: &AppState, message: impl Into<String>) -> Vec<UiCommand> {
    vec![
        UiCommand::UpdateOrphanFiles(dialog_json(state)),
        UiCommand::Menus(render::menus_json(state)),
        UiCommand::Status(render::status_json(state)),
        UiCommand::NotifyMessage(message.into()),
    ]
}

fn notify(state: &AppState, message: impl Into<String>) -> Vec<UiCommand> {
    vec![
        UiCommand::NotifyMessage(message.into()),
        UiCommand::UpdateOrphanFiles(dialog_json(state)),
        UiCommand::Menus(render::menus_json(state)),
        UiCommand::Status(render::status_json(state)),
    ]
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::gui::app::AppState;

    fn temp_directory(label: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_orphans_{label}_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    fn test_state(directory: &Path) -> AppState {
        let _ = crate::client_options::register_serialisable_type();
        let controller = crate::controller::Controller::new(directory).unwrap();
        controller.init_model().unwrap();

        AppState::new(Arc::new(controller))
    }

    fn storage_root(state: &AppState) -> PathBuf {
        let path = state.controller.files_manager().locations()[0].path.clone();
        std::fs::create_dir_all(&path).unwrap();

        path
    }

    fn write_known_file(state: &AppState, hash: [u8; 32]) -> PathBuf {
        let root = storage_root(state);
        let id = crate::db::master::get_hash_id(state.controller.db().db(), &hash).unwrap();
        assert!(id > 0);
        let path = root.join(format!(
            "{}.png",
            hash.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ));
        std::fs::write(&path, b"present").unwrap();

        path
    }

    #[test]
    fn scanning_finds_only_unknown_hashes() {
        let directory = temp_directory("scan");
        let state = test_state(&directory);

        let known = write_known_file(&state, [0x11; 32]);
        let root = storage_root(&state);
        std::fs::create_dir_all(root.join("f01")).unwrap();
        let orphan = root.join("f01").join(format!("{}.png", "ab".repeat(32)));
        std::fs::write(&orphan, b"surplus").unwrap();
        let thumbnail = root.join(format!("{}.thumbnail", "cd".repeat(32)));
        std::fs::write(&thumbnail, b"thumb").unwrap();
        let sidecar = root.join("notes.txt");
        std::fs::write(&sidecar, b"keep me").unwrap();

        let orphans = scan(&state).unwrap();
        let paths: Vec<&Path> = orphans.iter().map(|orphan| orphan.path.as_path()).collect();

        assert_eq!(orphans.len(), 2, "{paths:?}");
        assert!(!paths.contains(&known.as_path()));
        assert!(paths.contains(&orphan.as_path()));
        assert!(paths.contains(&thumbnail.as_path()));
        assert!(orphans.iter().any(|orphan| orphan.thumbnail));

        let value: Value = serde_json::from_str(&dialog_json(&state)).unwrap();
        assert_eq!(value["orphans"].as_array().unwrap().len(), 2);
        assert_eq!(value["files"], 1);
        assert_eq!(value["thumbnails"], 1);

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn deleting_removes_orphans_and_keeps_everything_else() {
        let directory = temp_directory("delete");
        let state = test_state(&directory);

        let known = write_known_file(&state, [0x22; 32]);
        let root = storage_root(&state);
        let orphan = root.join(format!("{}.png", "ef".repeat(32)));
        std::fs::write(&orphan, b"surplus").unwrap();

        let payload =
            json!({"paths": [orphan.to_string_lossy(), known.to_string_lossy()]}).to_string();
        let commands = delete(&state, &payload);

        assert!(commands.iter().any(
            |command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("deleted 1") && message.contains("skipped"))
        ), "{commands:?}");
        assert!(!orphan.exists());
        assert!(known.exists());

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn paths_outside_the_storage_are_rejected() {
        let directory = temp_directory("outside");
        let state = test_state(&directory);

        let outside = directory.join(format!("{}.png", "99".repeat(32)));
        std::fs::write(&outside, b"not storage").unwrap();

        let payload = json!({"paths": [outside.to_string_lossy()]}).to_string();
        delete(&state, &payload);

        assert!(
            outside.exists(),
            "a file outside the storage must not be deleted"
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn moving_sorts_files_and_thumbnails_into_subdirectories() {
        let directory = temp_directory("move");
        let state = test_state(&directory);

        let root = storage_root(&state);
        let orphan = root.join(format!("{}.png", "77".repeat(32)));
        let thumbnail = root.join(format!("{}.thumbnail", "77".repeat(32)));
        std::fs::write(&orphan, b"surplus").unwrap();
        std::fs::write(&thumbnail, b"thumb").unwrap();
        let target = directory.join("recovered");

        let payload = json!({
            "paths": [orphan.to_string_lossy(), thumbnail.to_string_lossy()],
            "target": target.to_string_lossy(),
        })
        .to_string();
        let commands = move_to(&state, &payload);

        assert!(commands.iter().any(
            |command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("moved 2"))
        ), "{commands:?}");
        assert!(!orphan.exists());
        assert!(!thumbnail.exists());
        assert!(target
            .join("files")
            .join(orphan.file_name().unwrap())
            .exists());
        assert!(target
            .join("thumbnails")
            .join(thumbnail.file_name().unwrap())
            .exists());

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn stored_names_are_parsed_conservatively() {
        let hash = "ab".repeat(32);

        assert_eq!(
            parse_stored_name(&format!("{hash}.png")),
            Some((hash.clone(), false))
        );
        assert_eq!(
            parse_stored_name(&format!("{hash}.thumbnail")),
            Some((hash, true))
        );
        assert_eq!(parse_stored_name("notes.txt"), None);
        assert_eq!(parse_stored_name(&format!("{}.png", "ab".repeat(31))), None);
    }
}
