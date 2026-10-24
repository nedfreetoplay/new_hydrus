//! Import folder model from `hydrus/client/importing/ClientImportLocal.py`.

use std::path::{Path, PathBuf};

use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult, unknown};

use crate::client_constants;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFolderAction {
    Delete,
    Ignore,
    Move,
}

impl ImportFolderAction {
    pub fn as_code(self) -> i32 {
        match self {
            Self::Delete => client_constants::IMPORT_FOLDER_DELETE,
            Self::Ignore => client_constants::IMPORT_FOLDER_IGNORE,
            Self::Move => client_constants::IMPORT_FOLDER_MOVE,
        }
    }

    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            client_constants::IMPORT_FOLDER_DELETE => Some(Self::Delete),
            client_constants::IMPORT_FOLDER_IGNORE => Some(Self::Ignore),
            client_constants::IMPORT_FOLDER_MOVE => Some(Self::Move),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFolder {
    pub name: String,
    pub path: PathBuf,
    pub actions: ImportFolderAction,
    pub action_locations: Option<PathBuf>,
    pub period: i64,
    pub check_regularly: bool,
    pub paused: bool,
    pub search_subdirectories: bool,
    pub last_check_time: i64,
    pub last_modified_time_skip_period: i64,
}

impl ImportFolder {
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>, period: i64) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            actions: ImportFolderAction::Ignore,
            action_locations: None,
            period,
            check_regularly: true,
            paused: false,
            search_subdirectories: true,
            last_check_time: 0,
            last_modified_time_skip_period: 0,
        }
    }

    pub fn get_next_work_time(&self) -> i64 {
        self.last_check_time + self.period
    }

    pub fn is_due(&self, now: i64) -> bool {
        !self.paused && self.check_regularly && now >= self.get_next_work_time()
    }

    pub fn pause_play(&mut self) {
        self.paused = !self.paused;
    }

    pub fn mark_checked(&mut self, now: i64) {
        self.last_check_time = now;
    }

    pub fn get_search_subdirectories(&self) -> bool {
        self.search_subdirectories
    }

    pub fn set_search_subdirectories(&mut self, value: bool) {
        self.search_subdirectories = value;
    }

    pub fn set_last_modified_time_skip_period(&mut self, value: i64) {
        self.last_modified_time_skip_period = value;
    }

    pub fn to_tuple(&self) -> (String, PathBuf, bool, bool) {
        (
            self.name.clone(),
            self.path.clone(),
            self.check_regularly,
            self.paused,
        )
    }

    /// Lists importable files, skipping recently modified ones.
    pub fn get_files_to_import(&self, now: i64) -> Vec<PathBuf> {
        let mut files = Vec::new();
        collect_files(
            &self.path,
            self.search_subdirectories,
            self.last_modified_time_skip_period,
            now,
            &mut files,
        );
        files.sort();

        files
    }
}

fn collect_files(
    directory: &Path,
    recursive: bool,
    skip_period: i64,
    now: i64,
    files: &mut Vec<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        if metadata.is_dir() {
            if recursive {
                collect_files(&path, recursive, skip_period, now, files);
            }

            continue;
        }

        if !metadata.is_file() {
            continue;
        }

        if skip_period > 0 {
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0);

            if modified > now - skip_period {
                continue;
            }
        }

        files.push(path);
    }
}

#[derive(Debug, Clone)]
pub struct HddImport {
    pub folder: ImportFolder,
    pub paths: Vec<PathBuf>,
    pub current: Option<usize>,
}

impl HddImport {
    pub fn new(folder: ImportFolder) -> Self {
        Self {
            folder,
            paths: Vec::new(),
            current: None,
        }
    }

    /// Scans the folder and returns how many files were queued.
    pub fn scan(&mut self, now: i64) -> usize {
        self.paths = self.folder.get_files_to_import(now);
        self.current = None;
        self.folder.mark_checked(now);

        self.paths.len()
    }

    pub fn pop_next(&mut self) -> Option<PathBuf> {
        if self.paths.is_empty() {
            return None;
        }

        self.current = Some(0);

        Some(self.paths.remove(0))
    }

    /// Applies the folder's post-import action to the current file.
    pub fn apply_action(&mut self, path: &Path) -> HydrusResult<()> {
        match self.folder.actions {
            ImportFolderAction::Ignore => Ok(()),
            ImportFolderAction::Delete => std::fs::remove_file(path)
                .map_err(|error| unknown(format!("could not delete {}: {error}", path.display()))),
            ImportFolderAction::Move => {
                let destination_directory =
                    self.folder.action_locations.clone().ok_or_else(|| {
                        HydrusException::new(
                            ErrorKind::DirectoryMissing,
                            "import folder move action has no destination",
                        )
                    })?;

                let file_name = path.file_name().ok_or_else(|| {
                    HydrusException::new(ErrorKind::Name, "imported file has no file name")
                })?;

                std::fs::create_dir_all(&destination_directory).map_err(|error| {
                    unknown(format!(
                        "could not create {}: {error}",
                        destination_directory.display()
                    ))
                })?;

                let mut destination = destination_directory.join(file_name);
                let mut counter = 1;

                while destination.exists() {
                    let stem = path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("file");
                    let extension = path.extension().and_then(|extension| extension.to_str());
                    let name = match extension {
                        Some(extension) => format!("{stem} ({counter}).{extension}"),
                        None => format!("{stem} ({counter})"),
                    };

                    destination = destination_directory.join(name);
                    counter += 1;
                }

                std::fs::rename(path, &destination).map_err(|error| {
                    unknown(format!(
                        "could not move {} to {}: {error}",
                        path.display(),
                        destination.display()
                    ))
                })
            }
        }
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct ImportFoldersManager {
    folders: Vec<ImportFolder>,
}

impl ImportFoldersManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_folder(&mut self, folder: ImportFolder) {
        self.folders.push(folder);
    }

    pub fn remove_folder(&mut self, name: &str) -> bool {
        let previous_len = self.folders.len();
        self.folders.retain(|folder| folder.name != name);
        previous_len != self.folders.len()
    }

    pub fn get_folder(&self, name: &str) -> Option<&ImportFolder> {
        self.folders.iter().find(|folder| folder.name == name)
    }

    pub fn get_folder_mut(&mut self, name: &str) -> Option<&mut ImportFolder> {
        self.folders.iter_mut().find(|folder| folder.name == name)
    }

    pub fn get_folders(&self) -> &[ImportFolder] {
        &self.folders
    }

    pub fn get_names(&self) -> Vec<&str> {
        self.folders
            .iter()
            .map(|folder| folder.name.as_str())
            .collect()
    }

    pub fn get_folder_name_that_is_due(&self, now: i64) -> Option<&str> {
        self.folders
            .iter()
            .filter(|folder| folder.is_due(now))
            .min_by_key(|folder| folder.get_next_work_time())
            .map(|folder| folder.name.as_str())
    }

    pub fn get_time_until_next_work(&self, now: i64) -> Option<i64> {
        self.folders
            .iter()
            .filter(|folder| !folder.paused && folder.check_regularly)
            .map(|folder| (folder.get_next_work_time() - now).max(0))
            .min()
    }

    pub fn len(&self) -> usize {
        self.folders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.folders.is_empty()
    }
}

#[cfg(test)]
mod tests;
