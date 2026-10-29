//! Import folders page and periodic scanning.
//!
//! The filesystem walk lives in `importing::import_folders`; this module owns
//! the GUI entries, their JSON file and the scan-scheduling glue to the async
//! import worker.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use hydrus_core::hydrus_constants::ServiceType;
use hydrus_core::hydrus_time;
use serde::{Deserialize, Serialize};

use crate::importing::import_folders::{ImportFolder, ImportFolderAction};
use crate::importing::options::FilenameTaggingOptions;

use super::model::ImportFoldersPage;
use super::worker::AsyncJob;
use super::{AppState, PageNode, UiCommand, render};

const FILENAME: &str = "import folders.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFolderEntry {
    pub name: String,
    pub path: PathBuf,
    #[serde(default = "default_period")]
    pub period: i64,
    #[serde(default)]
    pub paused: bool,
    #[serde(default = "default_true")]
    pub check_regularly: bool,
    #[serde(default = "default_true")]
    pub search_subdirectories: bool,
    #[serde(default)]
    pub last_check: i64,
    #[serde(default)]
    pub last_modified_skip: i64,
    #[serde(default)]
    pub last_imported: usize,
    #[serde(default)]
    pub status: String,
    /// What happens to a file after a successful import: `ignore`, `move` or
    /// `delete` (mirrors Hydrus' import folder actions).
    #[serde(default = "default_action")]
    pub action: String,
    #[serde(default)]
    pub action_path: Option<PathBuf>,
    /// Byte size limits; 0 disables the check.
    #[serde(default)]
    pub min_size: u64,
    #[serde(default)]
    pub max_size: u64,
    /// Comma/space separated extension allow list, without dots. Empty means
    /// every extension is accepted.
    #[serde(default)]
    pub extensions: String,
    /// Hex tag service key -> filename tagging options. Tags are added to that
    /// local tag service after a successful import.
    #[serde(default)]
    pub filename_tagging: BTreeMap<String, FilenameTaggingOptions>,
}

fn default_action() -> String {
    "ignore".to_string()
}

fn default_period() -> i64 {
    300
}

fn default_true() -> bool {
    true
}

impl ImportFolderEntry {
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>, period: i64) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            period: period.max(60),
            paused: false,
            check_regularly: true,
            search_subdirectories: true,
            last_check: 0,
            last_modified_skip: 0,
            last_imported: 0,
            status: "ready".to_string(),
            action: default_action(),
            action_path: None,
            min_size: 0,
            max_size: 0,
            extensions: String::new(),
            filename_tagging: BTreeMap::new(),
        }
    }

    pub fn to_import_folder(&self) -> ImportFolder {
        let mut folder = ImportFolder::new(self.name.clone(), self.path.clone(), self.period);
        folder.paused = self.paused;
        folder.check_regularly = self.check_regularly;
        folder.search_subdirectories = self.search_subdirectories;
        folder.last_check_time = self.last_check;
        folder.last_modified_time_skip_period = self.last_modified_skip;
        folder.actions = ImportFolderAction::from_code(match self.action.as_str() {
            "move" => crate::client_constants::IMPORT_FOLDER_MOVE,
            "delete" => crate::client_constants::IMPORT_FOLDER_DELETE,
            _ => crate::client_constants::IMPORT_FOLDER_IGNORE,
        })
        .unwrap_or(ImportFolderAction::Ignore);
        folder.action_locations = self.action_path.clone();
        folder
    }

    /// Applies the user's import filters to one candidate file.
    pub fn matches_filters(&self, path: &Path) -> bool {
        let Ok(metadata) = std::fs::metadata(path) else {
            return false;
        };

        if !metadata.is_file() {
            return false;
        }

        let size = metadata.len();

        if self.min_size > 0 && size < self.min_size {
            return false;
        }

        if self.max_size > 0 && size > self.max_size {
            return false;
        }

        let extensions = extension_list(&self.extensions);

        if !extensions.is_empty() {
            let extension = path
                .extension()
                .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();

            if !extensions.iter().any(|allowed| allowed == &extension) {
                return false;
            }
        }

        true
    }

    pub fn is_due(&self, now: i64) -> bool {
        !self.paused
            && self.check_regularly
            && now >= self.last_check + self.period.max(60)
            && self.path.is_dir()
    }
}

pub fn load(path: &Path) -> Vec<ImportFolderEntry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save(path: &Path, folders: &[ImportFolderEntry]) -> std::io::Result<()> {
    let dump = serde_json::to_string_pretty(folders).map_err(std::io::Error::other)?;
    std::fs::write(path, dump)
}

pub fn add(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
        return vec![UiCommand::NotifyMessage(
            "invalid import folder payload".to_string(),
        )];
    };

    let path = value
        .get("path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();

    if path.is_empty() {
        return vec![UiCommand::NotifyMessage(
            "choose an import folder first".to_string(),
        )];
    }

    let path = PathBuf::from(path);

    if !path.is_dir() {
        return vec![UiCommand::NotifyMessage(format!(
            "{} is not a directory",
            path.display()
        ))];
    }

    let name = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    let period = value
        .get("period")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(300)
        .max(60);

    let existing = state
        .import_folders
        .iter()
        .find(|folder| folder.name == name)
        .cloned();
    let mut entry =
        existing.unwrap_or_else(|| ImportFolderEntry::new(name.clone(), path.clone(), period));
    entry.name = name.clone();
    entry.path = path;
    entry.period = period;
    apply_payload(&mut entry, &value);

    state.import_folders.retain(|folder| folder.name != name);
    state.import_folders.push(entry);
    state.message = format!("import folder '{name}' added");
    let _ = persist(state);

    page_commands(state)
}

/// Opens the edit dialog for one folder. An empty payload opens a new folder
/// dialog with sensible defaults.
pub fn edit(state: &AppState, payload: &str) -> Vec<UiCommand> {
    let name = payload.trim();
    let existing = state
        .import_folders
        .iter()
        .find(|folder| folder.name == name)
        .cloned();

    let folder = match existing {
        Some(folder) => folder_json(&folder),
        None => folder_json(&ImportFolderEntry::new("", "", 300)),
    };

    let tag_services: Vec<serde_json::Value> = state
        .controller
        .services()
        .get_services(Some(&[ServiceType::LocalTag]))
        .into_iter()
        .map(|service| {
            serde_json::json!({
                "key": super::query::hex(&service.service_key),
                "name": service.name,
            })
        })
        .collect();

    vec![UiCommand::ImportFolderDialog(
        serde_json::json!({
            "folder": folder,
            "existing": !name.is_empty(),
            "tag_services": tag_services,
        })
        .to_string(),
    )]
}

/// Saves the values produced by the import folder dialog, creating or
/// updating the entry named by `original`.
pub fn save_action(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
        return vec![UiCommand::NotifyMessage(
            "invalid import folder payload".to_string(),
        )];
    };

    let path_text = value
        .get("path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();

    if path_text.is_empty() {
        return vec![UiCommand::NotifyMessage(
            "choose a folder first".to_string(),
        )];
    }

    let path = PathBuf::from(path_text);

    if !path.is_dir() {
        return vec![UiCommand::NotifyMessage(format!(
            "{} is not a directory",
            path.display()
        ))];
    }

    let original = value
        .get("original")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let name = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    let mut entry = state
        .import_folders
        .iter()
        .find(|folder| folder.name == original)
        .cloned()
        .unwrap_or_else(|| ImportFolderEntry::new(name.clone(), path.clone(), 300));

    entry.name = name.clone();
    entry.path = path;
    entry.period = value
        .get("period")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(entry.period)
        .max(60);
    apply_payload(&mut entry, &value);

    state
        .import_folders
        .retain(|folder| folder.name != original && folder.name != name);
    state.import_folders.push(entry);
    state.import_folders.sort_by(|a, b| a.name.cmp(&b.name));
    state.message = format!("import folder '{name}' saved");
    let _ = persist(state);

    page_commands(state)
}

fn apply_payload(entry: &mut ImportFolderEntry, value: &serde_json::Value) {
    if let Some(check) = value
        .get("check_regularly")
        .and_then(serde_json::Value::as_bool)
    {
        entry.check_regularly = check;
    }

    if let Some(subdirectories) = value
        .get("search_subdirectories")
        .and_then(serde_json::Value::as_bool)
    {
        entry.search_subdirectories = subdirectories;
    }

    if let Some(action) = value.get("action").and_then(serde_json::Value::as_str) {
        entry.action = match action {
            "move" | "delete" => action.to_string(),
            _ => "ignore".to_string(),
        };
    }

    if let Some(path) = value.get("action_path").and_then(serde_json::Value::as_str) {
        let path = path.trim();
        entry.action_path = (!path.is_empty()).then(|| PathBuf::from(path));
    }

    if let Some(size) = value.get("min_size").and_then(serde_json::Value::as_u64) {
        entry.min_size = size;
    }

    if let Some(size) = value.get("max_size").and_then(serde_json::Value::as_u64) {
        entry.max_size = size;
    }

    if let Some(extensions) = value.get("extensions").and_then(serde_json::Value::as_str) {
        entry.extensions = extensions.trim().to_string();
    }

    if let Some(tagging) = value
        .get("filename_tagging")
        .and_then(serde_json::Value::as_object)
    {
        entry.filename_tagging = tagging
            .iter()
            .filter_map(|(key, options)| {
                serde_json::from_value::<FilenameTaggingOptions>(options.clone())
                    .ok()
                    .filter(|options| !options.is_empty())
                    .map(|options| (key.clone(), options))
            })
            .collect();
    }

    if let Some(skip) = value
        .get("last_modified_skip")
        .and_then(serde_json::Value::as_i64)
    {
        entry.last_modified_skip = skip.max(0);
    }
}

fn extension_list(value: &str) -> Vec<String> {
    value
        .split(|character: char| character == ',' || character == ';' || character.is_whitespace())
        .map(|extension| {
            extension
                .trim()
                .trim_start_matches('.')
                .to_ascii_lowercase()
        })
        .filter(|extension| !extension.is_empty())
        .collect()
}

pub fn remove(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let name = payload.trim();
    let before = state.import_folders.len();
    state.import_folders.retain(|folder| folder.name != name);

    state.message = if state.import_folders.len() == before {
        format!("import folder '{name}' was not found")
    } else {
        format!("import folder '{name}' removed")
    };
    let _ = persist(state);

    page_commands(state)
}

pub fn toggle(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let name = payload.trim();

    if let Some(folder) = state
        .import_folders
        .iter_mut()
        .find(|folder| folder.name == name)
    {
        folder.paused = !folder.paused;
        folder.status = if folder.paused {
            "paused".to_string()
        } else {
            "ready".to_string()
        };
    }

    let _ = persist(state);
    page_commands(state)
}

pub fn check_now(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let name = payload.trim();
    let now = hydrus_time::get_now();

    let Some(folder) = state
        .import_folders
        .iter_mut()
        .find(|folder| folder.name == name)
    else {
        return vec![UiCommand::NotifyMessage(
            "import folder is missing".to_string(),
        )];
    };

    let candidates = folder.to_import_folder().get_files_to_import(now);
    let paths: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|path| folder.matches_filters(path))
        .collect();

    if paths.is_empty() {
        folder.status = "no new files".to_string();
        let _ = persist(state);
        return page_commands(state);
    }

    if let Err(error) = state.runtime.submit(AsyncJob::ImportPaths {
        paths: paths.clone(),
        folder: Some(folder.name.clone()),
        filename_tagging: folder
            .filename_tagging
            .iter()
            .filter_map(|(key, options)| super::query::unhex(key).map(|key| (key, options.clone())))
            .collect(),
    }) {
        folder.status = error;
        return page_commands(state);
    }

    folder.last_check = now;
    folder.last_imported = paths.len();
    folder.status = format!("queued {} files", paths.len());
    state.import_queue.running = true;
    state.import_queue.done = 0;
    state.import_queue.total = paths.len();
    state.import_queue.imported = 0;
    state.import_queue.failed = 0;
    state.import_queue.message = folder.status.clone();
    state.message = folder.status.clone();
    let _ = persist(state);

    page_commands(state)
}

pub fn tick(state: &mut AppState) -> Vec<UiCommand> {
    let now = hydrus_time::get_now();
    let due: Vec<String> = state
        .import_folders
        .iter()
        .filter(|folder| folder.is_due(now))
        .map(|folder| folder.name.clone())
        .collect();

    if due.is_empty() {
        return Vec::new();
    }

    for name in due {
        check_now(state, &name);
    }

    page_commands(state)
}

pub fn folder_json(folder: &ImportFolderEntry) -> serde_json::Value {
    serde_json::json!({
        "name": folder.name,
        "path": folder.path.display().to_string(),
        "period": folder.period,
        "paused": folder.paused,
        "check_regularly": folder.check_regularly,
        "search_subdirectories": folder.search_subdirectories,
        "last_check": folder.last_check,
        "last_imported": folder.last_imported,
        "status": folder.status,
        "exists": folder.path.is_dir(),
        "action": folder.action,
        "action_path": folder.action_path.as_ref().map(|path| path.display().to_string()).unwrap_or_default(),
        "min_size": folder.min_size,
        "max_size": folder.max_size,
        "extensions": folder.extensions,
        "last_modified_skip": folder.last_modified_skip,
        "filename_tagging": folder.filename_tagging,
    })
}

pub fn rules_json(state: &AppState, page_id: &str) -> String {
    let folders: Vec<serde_json::Value> = state.import_folders.iter().map(folder_json).collect();

    serde_json::json!({
        "page_id": page_id,
        "title": state.find_import_folders(page_id).map(|page| page.title.as_str()).unwrap_or("import folders"),
        "status": state.find_import_folders(page_id).map(|page| page.status.as_str()).unwrap_or(""),
        "folders": folders,
    })
    .to_string()
}

pub fn persist(state: &AppState) -> std::io::Result<()> {
    save(
        &state.controller.db_dir().join(FILENAME),
        &state.import_folders,
    )
}

pub fn create_page(state: &mut AppState, notebook_id: &str) -> Vec<UiCommand> {
    let id = state.new_id("p");
    let page = ImportFoldersPage::new(id.clone());

    let Some(notebook) = state.find_notebook_mut(notebook_id) else {
        return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
    };

    notebook.add(PageNode::ImportFolders(page));
    state.active = Some(id);
    page_commands(state)
}

fn page_commands(state: &AppState) -> Vec<UiCommand> {
    let ids: Vec<String> = state.import_folder_page_ids().into_iter().collect();

    let mut commands = vec![UiCommand::BuildPages(render::pages_json(state))];

    for id in ids {
        commands.push(UiCommand::UpdateImportFolders(
            id.clone(),
            rules_json(state, &id),
        ));
    }

    commands.push(UiCommand::Status(render::status_json(state)));
    commands
}

/// Applies the folder's post-import action to the files the worker finished.
/// `results` contains `(path, imported_ok)` pairs for this scan.
pub fn apply_post_import(
    state: &mut AppState,
    name: &str,
    results: &[(PathBuf, bool)],
) -> Vec<UiCommand> {
    let Some(index) = state
        .import_folders
        .iter()
        .position(|folder| folder.name == name)
    else {
        return Vec::new();
    };

    let action = state.import_folders[index].action.clone();
    let action_path = state.import_folders[index].action_path.clone();
    let mut imported = 0usize;
    let mut moved = 0usize;
    let mut deleted = 0usize;
    let mut failed = 0usize;

    for (path, ok) in results {
        if !ok {
            continue;
        }

        imported += 1;

        match action.as_str() {
            "move" => {
                let Some(directory) = action_path.as_ref() else {
                    failed += 1;
                    continue;
                };

                match move_file(path, directory) {
                    Ok(_) => moved += 1,
                    Err(_) => failed += 1,
                }
            }
            "delete" => match std::fs::remove_file(path) {
                Ok(()) => deleted += 1,
                Err(_) => failed += 1,
            },
            _ => {}
        }
    }

    let folder = &mut state.import_folders[index];
    folder.last_imported = imported;

    let action_note = match action.as_str() {
        "move" => format!(", {moved} moved"),
        "delete" => format!(", {deleted} deleted"),
        _ => String::new(),
    };

    folder.status = if failed > 0 {
        format!("imported {imported}{action_note}, {failed} action errors")
    } else {
        format!("imported {imported}{action_note}")
    };
    let status = folder.status.clone();
    state.message = format!("import folder '{name}': {status}");

    let _ = persist(state);

    page_commands(state)
}

fn move_file(source: &Path, directory: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(directory)?;

    let file_name = source.file_name().unwrap_or_default().to_string_lossy();
    let mut target = directory.join(file_name.as_ref());
    let path = Path::new(file_name.as_ref());
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let extension = path
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();
    let mut counter = 1;

    while target.exists() {
        target = directory.join(format!("{stem} ({counter}){extension}"));
        counter += 1;
    }

    match std::fs::rename(source, &target) {
        Ok(()) => Ok(target),
        Err(_) => {
            std::fs::copy(source, &target)?;
            std::fs::remove_file(source)?;
            Ok(target)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_become_due_after_their_period() {
        let mut folder = ImportFolderEntry::new("incoming", ".", 300);
        folder.last_check = 1_000;

        assert!(!folder.is_due(1_299));
        assert!(folder.is_due(1_300));

        folder.paused = true;
        assert!(!folder.is_due(9_999));
    }

    #[test]
    fn entries_convert_to_the_scanner_model() {
        let mut entry = ImportFolderEntry::new("incoming", ".", 120);
        entry.search_subdirectories = false;
        entry.last_modified_skip = 60;

        let folder = entry.to_import_folder();

        assert_eq!(folder.name, "incoming");
        assert_eq!(folder.period, 120);
        assert!(!folder.search_subdirectories);
        assert_eq!(folder.last_modified_time_skip_period, 60);
    }

    #[test]
    fn extension_and_size_filters_match_only_allowed_files() {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_import_filter_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).unwrap();

        let png = directory.join("keep.PNG");
        let text = directory.join("skip.txt");
        std::fs::write(&png, b"12345").unwrap();
        std::fs::write(&text, b"12345").unwrap();

        let mut entry = ImportFolderEntry::new("incoming", &directory, 300);
        entry.extensions = "png, .jpg".to_string();
        entry.min_size = 2;
        entry.max_size = 8;

        assert!(entry.matches_filters(&png));
        assert!(!entry.matches_filters(&text));

        entry.min_size = 6;
        assert!(!entry.matches_filters(&png));

        drop(entry);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn moving_a_file_avoids_overwriting_an_existing_name() {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_import_move_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let source = directory.join("source");
        let target = directory.join("target");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        let source_file = source.join("image.png");
        std::fs::write(&source_file, b"one").unwrap();
        std::fs::write(target.join("image.png"), b"two").unwrap();

        let moved = move_file(&source_file, &target).unwrap();

        assert!(!source_file.exists());
        assert_eq!(moved, target.join("image (1).png"));
        assert_eq!(std::fs::read(&moved).unwrap(), b"one");

        let _ = std::fs::remove_dir_all(directory);
    }
}

#[test]
fn filename_tagging_round_trips_through_the_folder_payload() {
    let mut entry = ImportFolderEntry::new("incoming", ".", 60);

    let payload = serde_json::json!({
        "filename_tagging": {
            "abcd": {
                "tags_for_all": ["test tag"],
                "add_filename": true,
                "filename_namespace": "filename",
                "directories": {"0": {"enabled": true, "namespace": "artist"}},
                "quick_namespaces": [],
                "regexes": []
            }
        }
    });

    apply_payload(&mut entry, &payload);

    assert_eq!(entry.filename_tagging.len(), 1);
    assert!(entry.filename_tagging["abcd"].add_filename);
    assert_eq!(entry.filename_tagging["abcd"].tags_for_all, ["test tag"]);

    let json = folder_json(&entry);
    let loaded: BTreeMap<String, FilenameTaggingOptions> =
        serde_json::from_value(json["filename_tagging"].clone()).unwrap();

    assert_eq!(loaded, entry.filename_tagging);
}

#[test]
fn empty_filename_tagging_rules_are_not_persisted() {
    let mut entry = ImportFolderEntry::new("incoming", ".", 60);

    apply_payload(
        &mut entry,
        &serde_json::json!({"filename_tagging": {"abcd": {}}}),
    );

    assert!(entry.filename_tagging.is_empty());
}
