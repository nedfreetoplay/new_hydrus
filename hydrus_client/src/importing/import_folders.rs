//! Import folder model from `hydrus/client/importing/ClientImportLocal.py`.

use std::path::PathBuf;

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
        self.folders
            .iter_mut()
            .find(|folder| folder.name == name)
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
mod tests {
    use super::*;

    #[test]
    fn folders_become_due_after_their_period() {
        let mut folder = ImportFolder::new("inbox", "C:/inbox", 60);
        folder.mark_checked(100);

        assert!(!folder.is_due(159));
        assert!(folder.is_due(160));
    }

    #[test]
    fn paused_folders_are_not_due() {
        let mut folder = ImportFolder::new("inbox", "C:/inbox", 60);
        folder.mark_checked(0);
        folder.pause_play();

        assert!(!folder.is_due(1000));
    }

    #[test]
    fn manager_picks_the_earliest_due_folder() {
        let mut manager = ImportFoldersManager::new();
        let mut first = ImportFolder::new("first", "C:/a", 100);
        first.mark_checked(0);
        let mut second = ImportFolder::new("second", "C:/b", 50);
        second.mark_checked(0);
        manager.add_folder(first);
        manager.add_folder(second);

        assert_eq!(manager.get_folder_name_that_is_due(60), Some("second"));
        assert_eq!(manager.get_time_until_next_work(60), Some(0));
    }

    #[test]
    fn folders_can_be_removed_by_name() {
        let mut manager = ImportFoldersManager::new();
        manager.add_folder(ImportFolder::new("inbox", "C:/inbox", 60));

        assert!(manager.remove_folder("inbox"));
        assert!(manager.is_empty());
        assert!(!manager.remove_folder("inbox"));
    }
}
