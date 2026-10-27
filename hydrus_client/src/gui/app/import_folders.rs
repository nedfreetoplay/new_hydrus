//! Import folders page and periodic scanning.
//!
//! The filesystem walk lives in `importing::import_folders`; this module owns
//! the GUI entries, their JSON file and the scan-scheduling glue to the async
//! import worker.

use std::path::{Path, PathBuf};

use hydrus_core::hydrus_time;
use serde::{Deserialize, Serialize};

use crate::importing::import_folders::ImportFolder;

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
        }
    }

    pub fn to_import_folder(&self) -> ImportFolder {
        let mut folder = ImportFolder::new(self.name.clone(), self.path.clone(), self.period);
        folder.paused = self.paused;
        folder.check_regularly = self.check_regularly;
        folder.search_subdirectories = self.search_subdirectories;
        folder.last_check_time = self.last_check;
        folder.last_modified_time_skip_period = self.last_modified_skip;
        folder
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

    state
        .import_folders
        .retain(|folder| folder.path != path || folder.name != name);
    state
        .import_folders
        .push(ImportFolderEntry::new(name.clone(), path, period));
    state.message = format!("import folder '{name}' added");
    let _ = persist(state);

    page_commands(state)
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

    let paths = folder.to_import_folder().get_files_to_import(now);

    if paths.is_empty() {
        folder.status = "no new files".to_string();
        let _ = persist(state);
        return page_commands(state);
    }

    if let Err(error) = state.runtime.submit(AsyncJob::ImportPaths {
        paths: paths.clone(),
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

pub fn rules_json(state: &AppState, page_id: &str) -> String {
    let folders: Vec<serde_json::Value> = state
        .import_folders
        .iter()
        .map(|folder| {
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
            })
        })
        .collect();

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
}
