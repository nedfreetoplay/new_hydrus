//! Split out from the parent module.

use super::*;

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
