//! File layout from `hydrus/client/files/ClientFilesManager.py`.

use std::path::{Path, PathBuf};

use hydrus_core::hydrus_constants::Mime;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use hydrus_core::hydrus_paths;

use crate::files::{FilesStorageBaseLocation, FilesStorageSubfolder};

#[derive(Debug, Clone, Default)]
pub struct ClientFilesManager {
    locations: Vec<FilesStorageBaseLocation>,
    subfolders: Vec<FilesStorageSubfolder>,
}

impl ClientFilesManager {
    pub fn new(subfolders: Vec<FilesStorageSubfolder>) -> Self {
        let mut locations: Vec<FilesStorageBaseLocation> = Vec::new();

        for subfolder in &subfolders {
            if !locations.contains(&subfolder.base_location) {
                locations.push(subfolder.base_location.clone());
            }
        }

        Self {
            locations,
            subfolders,
        }
    }

    pub fn locations(&self) -> &[FilesStorageBaseLocation] {
        &self.locations
    }

    pub fn subfolders(&self) -> &[FilesStorageSubfolder] {
        &self.subfolders
    }

    pub fn is_empty(&self) -> bool {
        self.subfolders.is_empty()
    }

    pub fn make_sure_exists(&self) -> HydrusResult<()> {
        for location in &self.locations {
            hydrus_paths::make_sure_directory_exists(&location.path)?;
        }

        Ok(())
    }

    /// `get_file_path`: chooses a hashed subfolder that matches the prefix type.
    pub fn get_file_path(&self, hash: &[u8], mime: Mime, thumbnail: bool) -> HydrusResult<PathBuf> {
        let filename = if thumbnail {
            format!("{}.thumbnail", hex_encode(hash))
        } else {
            format!("{}.{}", hex_encode(hash), mime.extension().trim_start_matches('.'))
        };

        self.get_path_in_subfolder(hash, &filename, thumbnail)
    }

    pub fn get_thumbnail_path(&self, hash: &[u8]) -> HydrusResult<PathBuf> {
        self.get_file_path(hash, Mime::Png, true)
    }

    pub fn get_path_in_subfolder(
        &self,
        hash: &[u8],
        filename: &str,
        thumbnail: bool,
    ) -> HydrusResult<PathBuf> {
        let prefix = if thumbnail { "t" } else { "f" };
        let candidates: Vec<&FilesStorageSubfolder> = self
            .subfolders
            .iter()
            .filter(|subfolder| subfolder.prefix.starts_with(prefix))
            .collect();

        if candidates.is_empty() {
            return Err(HydrusException::new(
                ErrorKind::DirectoryMissing,
                "no file storage subfolders are configured",
            ));
        }

        let index = hash.first().copied().unwrap_or(0) as usize % candidates.len();

        Ok(candidates[index].get_file_path(filename))
    }

    pub fn get_location_paths(&self) -> Vec<&Path> {
        self.locations
            .iter()
            .map(|location| location.path.as_path())
            .collect()
    }

    pub fn has_location(&self, path: &Path) -> bool {
        self.locations
            .iter()
            .any(|location| location.path == path)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> ClientFilesManager {
        let base = FilesStorageBaseLocation::new("C:/hydrus/files", 1, None);
        let thumbs = FilesStorageBaseLocation::new("C:/hydrus/thumbs", 1, None);

        ClientFilesManager::new(vec![
            FilesStorageSubfolder::new("fa", base.clone()),
            FilesStorageSubfolder::new("fb", base),
            FilesStorageSubfolder::new("ta", thumbs.clone()),
            FilesStorageSubfolder::new("tb", thumbs),
        ])
    }

    #[test]
    fn file_paths_use_hash_prefix_and_mime_extension() {
        let path = manager().get_file_path(&[0xab, 0xcd], Mime::Png, false).unwrap();

        assert!(path.starts_with("C:/hydrus/files"));
        assert!(path.to_string_lossy().ends_with(".png"));
    }

    #[test]
    fn thumbnail_paths_use_thumbnail_subfolders() {
        let path = manager().get_thumbnail_path(&[0xab]).unwrap();

        assert!(path.starts_with("C:/hydrus/thumbs"));
        assert!(path.to_string_lossy().ends_with(".thumbnail"));
    }

    #[test]
    fn hash_chooses_a_stable_subfolder() {
        let manager = manager();
        let first = manager.get_file_path(&[5], Mime::Png, false).unwrap();
        let second = manager.get_file_path(&[5], Mime::Png, false).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn empty_manager_reports_missing_folders() {
        assert!(ClientFilesManager::default()
            .get_thumbnail_path(&[1])
            .is_err());
    }

    #[test]
    fn locations_are_deduplicated() {
        let manager = manager();

        assert_eq!(manager.locations().len(), 2);
        assert!(manager.has_location(Path::new("C:/hydrus/files")));
    }
}
