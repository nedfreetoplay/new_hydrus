//! Physical file storage locations from `hydrus/client/files/ClientFilesPhysical.py`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use hydrus_core::files::hydrus_files_physical_storage;
use hydrus_core::hydrus_paths;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FilesStorageBaseLocation {
    pub path: PathBuf,
    pub ideal_weight: i64,
    pub max_num_bytes: Option<u64>,
}

impl FilesStorageBaseLocation {
    pub fn new(path: impl Into<PathBuf>, ideal_weight: i64, max_num_bytes: Option<u64>) -> Self {
        Self {
            path: path.into(),
            ideal_weight,
            max_num_bytes,
        }
    }

    pub fn able_to_accept_subfolders(
        &self,
        current_num_bytes: u64,
        num_bytes_of_subfolder: u64,
    ) -> bool {
        self.has_no_upper_limit()
            || current_num_bytes + num_bytes_of_subfolder <= self.max_num_bytes.unwrap_or(u64::MAX)
    }

    pub fn needs_to_remove_subfolders(&self, current_num_bytes: u64) -> bool {
        self.max_num_bytes
            .is_some_and(|maximum| current_num_bytes > maximum)
    }

    pub fn has_no_upper_limit(&self) -> bool {
        self.max_num_bytes.is_none()
    }

    pub fn make_sure_exists(&self) -> bool {
        hydrus_paths::make_sure_directory_exists(&self.path).is_ok()
    }

    pub fn path_exists(&self, lazy_check_ok: bool) -> bool {
        if lazy_check_ok {
            true
        } else {
            self.path.exists()
        }
    }

    /// `STATICGetIdealWeights`
    pub fn static_get_ideal_weights(
        total_num_bytes_to_hold: u64,
        base_locations: &[Self],
    ) -> BTreeMap<PathBuf, f64> {
        let total_weight: i64 = base_locations.iter().map(|location| location.ideal_weight).sum();
        let mut weights = BTreeMap::new();

        for location in base_locations {
            let weight = if total_weight == 0 {
                0.0
            } else {
                total_num_bytes_to_hold as f64 * (location.ideal_weight as f64 / total_weight as f64)
            };

            weights.insert(location.path.clone(), weight);
        }

        weights
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FilesStorageSubfolder {
    pub prefix: String,
    pub base_location: FilesStorageBaseLocation,
}

impl FilesStorageSubfolder {
    pub fn new(prefix: impl Into<String>, base_location: FilesStorageBaseLocation) -> Self {
        Self {
            prefix: prefix.into(),
            base_location,
        }
    }

    pub fn get_file_path(&self, filename: &str) -> PathBuf {
        self.base_location.path.join(&self.prefix).join(filename)
    }

    pub fn get_prefix_directories_longest_first(&self) -> Vec<PathBuf> {
        let mut directories = Vec::new();
        let mut current = self.base_location.path.clone();

        directories.push(current.clone());

        for character in self.prefix.chars() {
            current = current.join(character.to_string());
            directories.push(current.clone());
        }

        directories.reverse();
        directories
    }

    pub fn is_for_files(&self) -> bool {
        self.prefix.starts_with('f')
    }

    pub fn get_normalised_weight(&self) -> f64 {
        self.base_location.ideal_weight as f64
    }

    pub fn make_sure_exists(&self) -> bool {
        hydrus_paths::make_sure_directory_exists(&self.get_file_path("")).is_ok()
    }

    pub fn path_exists(&self, lazy_check_ok: bool) -> bool {
        if lazy_check_ok {
            true
        } else {
            self.get_file_path("").exists()
        }
    }
}

/// Path of a file inside `f<prefix>` storage.
pub fn get_file_path(
    base_location: &Path,
    hash: &[u8],
    prefix_length: usize,
    filename: &str,
) -> PathBuf {
    let prefix = hydrus_files_physical_storage::get_prefix(hash, "f", prefix_length);
    base_location.join(prefix).join(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> FilesStorageBaseLocation {
        FilesStorageBaseLocation::new("C:/hydrus/files", 10, None)
    }

    #[test]
    fn unlimited_locations_accept_everything() {
        assert!(base().able_to_accept_subfolders(u64::MAX - 1, 1));
        assert!(!base().needs_to_remove_subfolders(u64::MAX));
        assert!(base().has_no_upper_limit());
    }

    #[test]
    fn limited_locations_respect_the_ceiling() {
        let location = FilesStorageBaseLocation::new("C:/hydrus/files", 10, Some(100));

        assert!(location.able_to_accept_subfolders(50, 50));
        assert!(!location.able_to_accept_subfolders(50, 51));
        assert!(location.needs_to_remove_subfolders(101));
    }

    #[test]
    fn ideal_weights_are_proportional() {
        let locations = [
            FilesStorageBaseLocation::new("a", 1, None),
            FilesStorageBaseLocation::new("b", 3, None),
        ];

        let weights = FilesStorageBaseLocation::static_get_ideal_weights(400, &locations);

        assert_eq!(weights[Path::new("a")], 100.0);
        assert_eq!(weights[Path::new("b")], 300.0);
    }

    #[test]
    fn subfolder_paths_match_hydrus_layout() {
        let subfolder = FilesStorageSubfolder::new("fa", base());
        let path = subfolder.get_file_path("abc");

        assert!(path.ends_with("fa/abc"));
        assert!(subfolder.is_for_files());
        assert_eq!(subfolder.get_prefix_directories_longest_first().len(), 3);
    }

    #[test]
    fn hashed_file_path_uses_core_prefix() {
        let path = get_file_path(Path::new("C:/hydrus/files"), &[0xab, 0xcd], 2, "file");

        assert!(path.ends_with("fab/file"));
    }
}
