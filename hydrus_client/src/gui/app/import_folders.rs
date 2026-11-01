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
use crate::gui::qt::api::ffi::{
    FolderPageState, FolderRow, ImportFolderEditorState, TagServiceRef, TaggingEntry,
};

const FILENAME: &str = "import folders.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFolderEntry {
    pub name: String,
    pub path: PathBuf,
    #[serde(default = "default_period")]
    pub period: i64,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub check_now: bool,
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
    3600
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
            check_now: false,
            check_regularly: true,
            search_subdirectories: true,
            last_check: 0,
            last_modified_skip: 60,
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

    vec![UiCommand::ImportFolderDialog(editor_state(state, name))]
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
            "You must enter a path to import from!".to_string(),
        )];
    }

    let path = PathBuf::from(path_text);

    let original = value
        .get("original")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let raw_name = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string();

    let taken: Vec<String> = state
        .import_folders
        .iter()
        .filter(|folder| folder.name != original)
        .map(|folder| folder.name.clone())
        .collect();
    let name = non_dupe_name(&raw_name, &taken);

    let mut entry = state
        .import_folders
        .iter()
        .find(|folder| folder.name == original)
        .cloned()
        .unwrap_or_else(|| ImportFolderEntry::new(name.clone(), path.clone(), 3600));

    entry.name = name.clone();
    entry.path = path;
    entry.period = value
        .get("period")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(entry.period)
        .max(180);
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

    if let Some(check_now) = value
        .get("check_now")
        .and_then(serde_json::Value::as_bool)
    {
        entry.check_now = check_now;
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
        entry.last_modified_skip = skip.max(1);
    }
}

pub fn extension_list(value: &str) -> Vec<String> {
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

/// Python's `GetNonDupeName`: `name (1)`, `name (2)`, ... against casefolded
/// existing names.
pub fn non_dupe_name(name: &str, taken: &[String]) -> String {
    let taken: std::collections::BTreeSet<String> =
        taken.iter().map(|name| name.to_lowercase()).collect();

    if !taken.contains(&name.to_lowercase()) {
        return name.to_string();
    }

    let mut index = 1;

    loop {
        let candidate = format!("{name} ({index})");

        if !taken.contains(&candidate.to_lowercase()) {
            return candidate;
        }

        index += 1;
    }
}

/// Splits a period into the days/hours/minutes shown by Python's time delta
/// editor.
pub fn period_components(seconds: i64) -> (i64, i64, i64) {
    let seconds = seconds.max(0);

    (
        seconds / 86_400,
        seconds % 86_400 / 3_600,
        seconds % 3_600 / 60,
    )
}

pub fn period_from_components(days: i64, hours: i64, minutes: i64) -> i64 {
    let seconds = days.max(0) * 86_400 + hours.clamp(0, 23) * 3_600 + minutes.clamp(0, 59) * 60;

    // Python's TimeDeltaButton is created with `min = 3 * 60`.
    seconds.max(180)
}

pub fn validate_folder_path(path: &str) -> Result<PathBuf, String> {
    let path = path.trim();

    if path.is_empty() {
        return Err("You must enter a path to import from!".to_string());
    }

    Ok(PathBuf::from(path))
}

pub fn pretty_period_public(seconds: i64) -> String {
    pretty_period(seconds)
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
        delete_after_success: false,
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
    let pretty_period = if folder.check_regularly {
        pretty_period(folder.period)
    } else {
        "not checking regularly".to_string()
    };

    serde_json::json!({
        "name": folder.name,
        "path": folder.path.display().to_string(),
        "period": folder.period,
        "pretty_period": pretty_period,
        "paused": folder.paused,
        "check_now": folder.check_now,
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

/// Python's `edit import folders` manager dialog.
pub fn manage(state: &AppState) -> Vec<UiCommand> {
    vec![UiCommand::ImportFoldersDialog(manager_state(state))]
}

pub fn pretty_period(seconds: i64) -> String {
    let seconds = seconds.max(60);

    if seconds % 86_400 == 0 {
        let days = seconds / 86_400;
        format!("{days} day{}", if days == 1 { "" } else { "s" })
    } else if seconds % 3_600 == 0 {
        let hours = seconds / 3_600;
        format!("{hours} hour{}", if hours == 1 { "" } else { "s" })
    } else if seconds % 60 == 0 {
        let minutes = seconds / 60;
        format!("{minutes} minute{}", if minutes == 1 { "" } else { "s" })
    } else {
        format!("{seconds} seconds")
    }
}

pub fn folder_page_state(state: &AppState, page_id: &str) -> FolderPageState {
    let folders = state
        .import_folders
        .iter()
        .map(|folder| {
            let action_text = if folder.action == "move" {
                format!(
                    "move -> {}",
                    folder
                        .action_path
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_default()
                )
            } else {
                folder.action.clone()
            };
            let mut filters = Vec::new();
            if !folder.extensions.trim().is_empty() {
                filters.push(format!("ext: {}", folder.extensions.trim()));
            }
            if folder.min_size > 0 || folder.max_size > 0 {
                filters.push(format!(
                    "{} - {} bytes",
                    folder.min_size,
                    if folder.max_size > 0 {
                        folder.max_size.to_string()
                    } else {
                        "any".to_string()
                    }
                ));
            }
            if folder.last_modified_skip > 0 {
                filters.push(format!("skip files newer than {} s", folder.last_modified_skip));
            }
            FolderRow {
                name: folder.name.clone(),
                path: folder.path.display().to_string(),
                status: folder.status.clone(),
                paused: folder.paused,
                period_text: format!("{} s", folder.period),
                last_queued: folder.last_imported.to_string(),
                extra: action_text,
                query_text: String::new(),
                filters_text: if filters.is_empty() {
                    "any".to_string()
                } else {
                    filters.join("; ")
                },
                mode_text: String::new(),
                phrase_text: String::new(),
                error_text: String::new(),
            }
        })
        .collect();

    FolderPageState {
        summary: state
            .find_import_folders(page_id)
            .map(|page| page.status.clone())
            .unwrap_or_default(),
        folders,
    }
}

/// Typed manager-dialog state (WS-15.2.4b): Rust formats the period label.
pub fn manager_state(state: &AppState) -> FolderPageState {
    let folders = state
        .import_folders
        .iter()
        .map(|folder| FolderRow {
            name: folder.name.clone(),
            path: folder.path.display().to_string(),
            status: String::new(),
            paused: folder.paused,
            period_text: if folder.check_regularly {
                pretty_period(folder.period)
            } else {
                "not checking regularly".to_string()
            },
            last_queued: String::new(),
            extra: String::new(),
            query_text: String::new(),
            filters_text: String::new(),
            mode_text: String::new(),
            phrase_text: String::new(),
            error_text: String::new(),
        })
        .collect();

    FolderPageState {
        summary: String::new(),
        folders,
    }
}

/// `filename tagging` summary, mirroring the C++ preview helper.
pub fn tagging_summary(options: &FilenameTaggingOptions) -> String {
    let mut statements = Vec::new();
    if !options.tags_for_all.is_empty() {
        statements.push(format!("{} tag(s) for all", options.tags_for_all.len()));
    }
    if options.add_filename {
        statements.push(format!("filename as '{}'", options.filename_namespace));
    }
    let directories = options.directories.values().filter(|rule| rule.enabled).count();
    if directories > 0 {
        statements.push(format!("{directories} directory rule(s)"));
    }
    if !options.quick_namespaces.is_empty() {
        statements.push(format!("{} namespace regex(es)", options.quick_namespaces.len()));
    }
    if !options.regexes.is_empty() {
        statements.push(format!("{} regex(es)", options.regexes.len()));
    }
    if statements.is_empty() {
        "no tags".to_string()
    } else {
        statements.join(", ")
    }
}

fn tagging_entry(
    key: &str,
    name: &str,
    options: &FilenameTaggingOptions,
) -> TaggingEntry {
    use crate::importing::options::FILENAME_TAGGING_DIRECTORIES;
    let mut dir_enabled = Vec::new();
    let mut dir_namespaces = Vec::new();
    for slot in FILENAME_TAGGING_DIRECTORIES {
        let rule = options.directories.get(slot);
        dir_enabled.push(rule.map(|rule| rule.enabled).unwrap_or(false));
        dir_namespaces.push(
            rule.map(|rule| rule.namespace.clone())
                .unwrap_or_default(),
        );
    }
    TaggingEntry {
        service_key: key.to_string(),
        service_name: name.to_string(),
        summary: tagging_summary(options),
        tags_for_all: options.tags_for_all.clone(),
        add_filename: options.add_filename,
        filename_namespace: options.filename_namespace.clone(),
        dir_enabled,
        dir_namespaces,
        quick_namespaces: options
            .quick_namespaces
            .iter()
            .map(|rule| rule.namespace.clone())
            .collect(),
        quick_regexes: options
            .quick_namespaces
            .iter()
            .map(|rule| rule.regex.clone())
            .collect(),
        regexes: options.regexes.clone(),
    }
}

/// Typed editor state for one import folder (WS-15.2.4b).
pub fn editor_state(state: &AppState, name: &str) -> ImportFolderEditorState {
    let existing = state.import_folders.iter().find(|folder| folder.name == name);
    let folder = existing.cloned().unwrap_or_else(|| ImportFolderEntry::new("import folder", "", 3600));
    let services: Vec<(String, String)> = state
        .controller
        .services()
        .get_services(Some(&[ServiceType::LocalTag]))
        .into_iter()
        .map(|service| (super::query::hex(&service.service_key), service.name.clone()))
        .collect();
    let name_to_label: std::collections::HashMap<String, String> =
        services.iter().map(|(key, name)| (key.clone(), name.clone())).collect();
    let mut tagging: Vec<TaggingEntry> = folder
        .filename_tagging
        .iter()
        .map(|(key, options)| {
            tagging_entry(
                key,
                name_to_label.get(key).map(String::as_str).unwrap_or(key),
                options,
            )
        })
        .collect();
    tagging.sort_by(|a, b| a.service_name.cmp(&b.service_name));
    ImportFolderEditorState {
        original: folder.name.clone(),
        name: folder.name.clone(),
        path: folder.path.display().to_string(),
        search_subdirectories: folder.search_subdirectories,
        paused: folder.paused,
        check_regularly: folder.check_regularly,
        period: folder.period,
        period_label: pretty_delta(folder.period),
        skip: folder.last_modified_skip,
        skip_label: pretty_delta(folder.last_modified_skip),
        check_now: folder.check_now,
        show_working_popup: true,
        publish_popup: true,
        publish_page: false,
        action: folder.action.clone(),
        action_path: folder
            .action_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        tagging,
        tag_services: services
            .into_iter()
            .map(|(key, name)| TagServiceRef { key, name })
            .collect(),
    }
}

/// Python `TimeDeltaToPrettyTimeDelta` (two units max); C++ only displays it.
pub fn pretty_delta(seconds: i64) -> String {
    if seconds <= 0 {
        return "0 seconds".to_string();
    }
    let mut remaining = seconds;
    let mut parts: Vec<String> = Vec::new();
    for (name, size) in [("day", 86_400_i64), ("hour", 3_600), ("minute", 60), ("second", 1)] {
        let quantity = remaining / size;
        remaining %= size;
        if quantity > 0 {
            parts.push(format!(
                "{quantity} {name}{}",
                if quantity > 1 { "s" } else { "" }
            ));
            if parts.len() == 2 {
                break;
            }
        } else if !parts.is_empty() {
            break;
        }
    }
    parts.join(" ")
}

#[cfg(test)]
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

    let mut commands = vec![UiCommand::BuildPages(render::pages(state))];

    for id in ids {
        commands.push(UiCommand::UpdateImportFolders(
            id.clone(),
            folder_page_state(state, &id),
        ));
    }

    // No-op in Qt when the manager dialog is closed.
    commands.push(UiCommand::UpdateImportFoldersDialog(manager_state(state)));
    commands.push(UiCommand::Status(render::status(state)));
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
    use crate::controller::Controller;

    fn testing_state(
        name: &str,
        folders: Vec<ImportFolderEntry>,
    ) -> (std::path::PathBuf, AppState) {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_gui_1524b_{name}_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let controller = std::sync::Arc::new(Controller::new(&directory).unwrap());
        controller.init_model().unwrap();
        let mut state = AppState::new(controller);
        state.import_folders = folders;
        (directory, state)
    }

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
    fn check_periods_are_pretty_printed() {
        let mut entry = ImportFolderEntry::new("incoming", ".", 300);
        assert_eq!(folder_json(&entry)["pretty_period"], "5 minutes");

        entry.check_regularly = false;
        assert_eq!(
            folder_json(&entry)["pretty_period"],
            "not checking regularly"
        );
    }

    #[test]
    fn typed_manager_rows_carry_pretty_periods() {
        let entry = ImportFolderEntry::new("incoming", ".", 300);
        let (directory, state) = testing_state("manager", vec![entry]);
        let manager = manager_state(&state);

        assert_eq!(manager.folders.len(), 1);
        assert_eq!(manager.folders[0].name, "incoming");
        assert_eq!(manager.folders[0].period_text, "5 minutes");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn typed_page_rows_carry_action_and_filters() {
        let mut entry = ImportFolderEntry::new("incoming", ".", 300);
        entry.action = "move".to_string();
        entry.action_path = Some(std::path::PathBuf::from("/tmp/done"));
        entry.extensions = "png".to_string();
        let (directory, state) = testing_state("page", vec![entry]);
        let page = folder_page_state(&state, "no-such-page");

        assert_eq!(page.folders.len(), 1);
        assert_eq!(page.folders[0].extra, "move -> /tmp/done");
        assert!(page.folders[0].filters_text.contains("ext: png"));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn tagging_summaries_list_every_rule_kind() {
        let mut options = FilenameTaggingOptions::default();
        options.tags_for_all = vec!["a".to_string()];
        options.add_filename = true;
        assert_eq!(
            tagging_summary(&options),
            "1 tag(s) for all, filename as 'filename'"
        );
        assert_eq!(tagging_summary(&FilenameTaggingOptions::default()), "no tags");
    }

    #[test]
    fn typed_deltas_show_two_units_max() {
        assert_eq!(pretty_delta(90_000), "1 day 1 hour");
        assert_eq!(pretty_delta(0), "0 seconds");
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

#[test]
fn import_folder_non_dupe_name_appends_counter() {
    let taken = vec!["import folder".to_string(), "import folder (1)".to_string()];
    assert_eq!(non_dupe_name("import folder", &taken), "import folder (2)");
    assert_eq!(non_dupe_name("new folder", &taken), "new folder");
}

#[test]
fn import_folder_period_helpers_round_trip() {
    assert_eq!(period_components(90_000), (1, 1, 0));
    assert_eq!(period_from_components(1, 1, 0), 90_000);
    assert_eq!(period_from_components(0, 0, 0), 180);
}

#[test]
fn import_folder_extension_list_parses() {
    assert_eq!(extension_list("png, .jpg; WEBP"), vec!["png", "jpg", "webp"]);
}

#[test]
fn import_folder_validate_path_rejects_empty() {
    assert!(validate_folder_path("").is_err());
    assert_eq!(validate_folder_path("C:/tmp").unwrap().to_string_lossy(), "C:/tmp");
}

#[test]
fn import_folder_pretty_period_formats() {
    assert_eq!(pretty_period(300), "5 minutes");
    assert_eq!(pretty_period(86_400), "1 day");
    assert_eq!(pretty_period(0), "1 minute");
}
