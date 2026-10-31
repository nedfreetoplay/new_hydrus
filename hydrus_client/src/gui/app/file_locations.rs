//! Repair file storage locations from
//! `hydrus/client/gui/panels/ClientGUIRepairFileSystemPanel.py`.
//!
//! The Python client asks the user to remap missing media/thumbnail
//! subdirectories during boot. In the Rust port the client always starts, so
//! the same panel is available from `database -> maintenance -> repair file
//! locations...`. Applying a repair writes the new subfolder rows, updates the
//! live files manager and creates any empty thumbnail directories the user
//! agreed to.

use std::path::{Path, PathBuf};

use hydrus_core::hydrus_paths;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::db::files_physical_storage;
use crate::files::{ClientFilesManager, FilesStorageBaseLocation, FilesStorageSubfolder};

use super::UiCommand;
use super::model::AppState;
use super::render;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairEntry {
    pub prefix: String,
    pub old_base: PathBuf,
    pub base: PathBuf,
    pub is_for_files: bool,
}

impl RepairEntry {
    fn subfolder(&self) -> FilesStorageSubfolder {
        FilesStorageSubfolder::new(&self.prefix, base_location(&self.base))
    }

    pub fn exists(&self) -> bool {
        self.subfolder().path_exists(false)
    }

    pub fn subfolder_path(&self) -> PathBuf {
        self.subfolder().path()
    }
}

#[derive(Debug, Clone, Default)]
pub struct RepairState {
    pub entries: Vec<RepairEntry>,
    pub message: String,
    pub error: String,
}

fn base_location(path: &Path) -> FilesStorageBaseLocation {
    FilesStorageBaseLocation::new(path, 0, None)
}

pub fn open(state: &mut AppState) -> Vec<UiCommand> {
    let manager = state.controller.files_manager();

    if manager.subfolders().is_empty() {
        return vec![UiCommand::NotifyMessage(
            "no file storage subfolders are configured".to_string(),
        )];
    }

    state.file_location_repair = RepairState {
        entries: manager
            .subfolders()
            .iter()
            .map(|subfolder| RepairEntry {
                prefix: subfolder.prefix.clone(),
                old_base: subfolder.base_location.path.clone(),
                base: subfolder.base_location.path.clone(),
                is_for_files: subfolder.is_for_files(),
            })
            .collect(),
        message: String::new(),
        error: String::new(),
    };

    vec![UiCommand::RepairFileLocationsDialog(dialog_json(state))]
}

pub fn set_locations(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return repair_error(state, "could not read the repair locations");
    };

    let updates: Vec<Value> = match value.get("updates") {
        Some(Value::Array(updates)) => updates.clone(),
        _ => vec![value],
    };

    let mut changed = 0usize;

    for update in updates {
        let Some(prefix) = update.get("prefix").and_then(Value::as_str) else {
            continue;
        };
        let Some(base) = update.get("base").and_then(Value::as_str) else {
            continue;
        };
        let base = base.trim();

        if base.is_empty() {
            continue;
        }

        if let Some(entry) = state
            .file_location_repair
            .entries
            .iter_mut()
            .find(|entry| entry.prefix == prefix)
        {
            if entry.base.as_path() != Path::new(base) {
                entry.base = PathBuf::from(base);
                changed += 1;
            }
        }
    }

    state.file_location_repair.error.clear();
    state.file_location_repair.message = format!("updated {changed} location(s)");

    vec![UiCommand::RepairFileLocationsDialog(dialog_json(state))]
}

/// `_AddLocation`: try a folder root and its `client_files`/`thumbnails`
/// children for every subfolder that is currently missing.
pub fn probe(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return repair_error(state, "could not read the probe location");
    };

    let Some(path) = value.get("path").and_then(Value::as_str).map(str::trim) else {
        return repair_error(state, "pick a folder to probe");
    };

    if path.is_empty() {
        return repair_error(state, "pick a folder to probe");
    }

    let root = PathBuf::from(path);
    let candidates = [
        root.clone(),
        root.join("client_files"),
        root.join("thumbnails"),
    ];
    let mut found = 0usize;

    for entry in &mut state.file_location_repair.entries {
        if entry.exists() {
            continue;
        }

        let mut best: Option<(PathBuf, usize)> = None;

        for candidate in &candidates {
            let subfolder = FilesStorageSubfolder::new(&entry.prefix, base_location(candidate));

            if !subfolder.path_exists(false) {
                continue;
            }

            // Prefer a client_files/thumbnails child over its parent when both
            // exist and the child matches the subfolder type: that is the more
            // specific base the user pointed at.
            let file_name = candidate
                .file_name()
                .map(|name| name.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            let score = if (file_name == "client_files" && entry.is_for_files)
                || (file_name == "thumbnails" && !entry.is_for_files)
            {
                2
            } else {
                1
            };

            if best
                .as_ref()
                .is_none_or(|(_, best_score)| score > *best_score)
            {
                best = Some((candidate.clone(), score));
            }
        }

        if let Some((candidate, _)) = best {
            entry.base = candidate;
            found += 1;
        }
    }

    state.file_location_repair.error.clear();
    state.file_location_repair.message = format!("found {found} subfolder(s) under {path}");

    vec![UiCommand::RepairFileLocationsDialog(dialog_json(state))]
}

pub fn apply(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let create_thumbnails = serde_json::from_str::<Value>(payload)
        .ok()
        .and_then(|value| value.get("create_thumbnails").and_then(Value::as_bool))
        .unwrap_or(true);

    if state.file_location_repair.entries.is_empty() {
        return repair_error(state, "there are no file locations to repair");
    }

    for entry in &state.file_location_repair.entries {
        if entry.exists() {
            continue;
        }

        if entry.is_for_files {
            let message = format!(
                "the file subfolder '{}' is still missing at {}",
                entry.prefix,
                entry.base.display()
            );

            return repair_error(state, message);
        }

        if !create_thumbnails {
            let message = format!(
                "the thumbnail subfolder '{}' is still missing at {}",
                entry.prefix,
                entry.base.display()
            );

            return repair_error(state, message);
        }
    }

    let mut subfolders = Vec::with_capacity(state.file_location_repair.entries.len());
    let mut create_error = None;

    for entry in &mut state.file_location_repair.entries {
        let subfolder = entry.subfolder();

        if !subfolder.path_exists(false) && (entry.is_for_files || create_thumbnails) {
            if let Err(error) = hydrus_paths::make_sure_directory_exists(&subfolder.path()) {
                create_error = Some(format!(
                    "could not create {}: {error}",
                    subfolder.path().display()
                ));
                break;
            }
        }

        subfolders.push(FilesStorageSubfolder::new(
            &entry.prefix,
            base_location(&entry.base),
        ));
    }

    if let Some(message) = create_error {
        return repair_error(state, message);
    }

    let old_locations = state.controller.files_manager().locations().to_vec();
    let mut locations: Vec<FilesStorageBaseLocation> = Vec::new();

    for subfolder in &subfolders {
        if locations
            .iter()
            .any(|location| location.path == subfolder.base_location.path)
        {
            continue;
        }

        let location = old_locations
            .iter()
            .find(|location| location.path == subfolder.base_location.path)
            .cloned()
            .unwrap_or_else(|| subfolder.base_location.clone());
        locations.push(location);
    }

    let db = state.controller.db().db();

    if let Err(error) = files_physical_storage::set_subfolders(db, &subfolders) {
        return repair_error(state, format!("could not save the subfolders: {error}"));
    }

    if let Err(error) = files_physical_storage::set_current_locations(db, &locations) {
        return repair_error(state, format!("could not save the locations: {error}"));
    }

    state
        .controller
        .set_files_manager(ClientFilesManager::new(subfolders));
    state.file_location_repair = RepairState::default();

    vec![
        UiCommand::CloseRepairFileLocations,
        UiCommand::NotifyMessage("file locations repaired".to_string()),
        UiCommand::Menus(render::menus_json(state)),
        UiCommand::Status(render::status(state)),
    ]
}

fn repair_error(state: &mut AppState, message: impl Into<String>) -> Vec<UiCommand> {
    state.file_location_repair.error = message.into();

    vec![UiCommand::RepairFileLocationsDialog(dialog_json(state))]
}

pub fn dialog_json(state: &AppState) -> String {
    let repair = &state.file_location_repair;
    let rows: Vec<Value> = repair
        .entries
        .iter()
        .map(|entry| {
            let exists = entry.exists();

            json!({
                "prefix": entry.prefix,
                "old_base": entry.old_base.display().to_string(),
                "base": entry.base.display().to_string(),
                "subfolder_path": entry.subfolder_path().display().to_string(),
                "is_for_files": entry.is_for_files,
                "exists": exists,
                "status": if exists { "ok!" } else { "missing" },
            })
        })
        .collect();
    let missing_files = repair
        .entries
        .iter()
        .filter(|entry| entry.is_for_files && !entry.exists())
        .count();
    let missing_thumbs = repair
        .entries
        .iter()
        .filter(|entry| !entry.is_for_files && !entry.exists())
        .count();

    json!({
        "title": "repair file locations",
        "rows": rows,
        "missing_files": missing_files,
        "missing_thumbs": missing_thumbs,
        "message": repair.message,
        "error": repair.error,
        "summary": format!(
            "{} subfolder(s): {} file path(s) missing, {} thumbnail path(s) missing",
            repair.entries.len(),
            missing_files,
            missing_thumbs,
        ),
    })
    .to_string()
}

#[cfg(test)]
mod tests;
