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

mod hdd;
pub use hdd::*;

#[cfg(test)]
mod tests;
