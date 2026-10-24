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
        if self.ideal_weight == 0 {
            return false;
        }

        self.has_no_upper_limit()
            || current_num_bytes + num_bytes_of_subfolder <= self.max_num_bytes.unwrap_or(u64::MAX)
    }

    pub fn eager_to_accept_subfolders(
        &self,
        current_normalised_weight: f64,
        total_ideal_weight: i64,
        weight_of_subfolder: f64,
        current_num_bytes: u64,
        num_bytes_of_subfolder: u64,
    ) -> bool {
        if !self.able_to_accept_subfolders(current_num_bytes, num_bytes_of_subfolder) {
            return false;
        }

        if total_ideal_weight == 0 {
            return false;
        }

        let ideal_normalised_weight = self.ideal_weight as f64 / total_ideal_weight as f64;

        current_normalised_weight + weight_of_subfolder <= ideal_normalised_weight
    }

    pub fn would_like_to_remove_subfolders(
        &self,
        current_normalised_weight: f64,
        total_ideal_weight: i64,
        weight_of_subfolder: f64,
    ) -> bool {
        if self.ideal_weight == 0 {
            return true;
        }

        if total_ideal_weight == 0 {
            return false;
        }

        let ideal_normalised_weight = self.ideal_weight as f64 / total_ideal_weight as f64;

        current_normalised_weight - weight_of_subfolder > ideal_normalised_weight
    }

    pub fn needs_to_remove_subfolders(&self, current_num_bytes: u64) -> bool {
        if self.ideal_weight == 0 {
            return true;
        }

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
        let total_weight: i64 = base_locations
            .iter()
            .map(|location| location.ideal_weight)
            .sum();
        let mut weights = BTreeMap::new();

        for location in base_locations {
            let weight = if total_weight == 0 {
                0.0
            } else {
                total_num_bytes_to_hold as f64
                    * (location.ideal_weight as f64 / total_weight as f64)
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

    pub fn get_our_subfolders(&self) -> Vec<String> {
        if self.prefix.len() <= 1 {
            return Vec::new();
        }

        let first_char = self.prefix[..1].to_string();
        let hex_prefix = &self.prefix[1..];
        let mut subfolders: Vec<String> = hex_prefix
            .as_bytes()
            .chunks(2)
            .map(|chunk| String::from_utf8_lossy(chunk).to_string())
            .collect();

        if let Some(first) = subfolders.first_mut() {
            *first = format!("{first_char}{first}");
        }

        subfolders
    }

    pub fn path(&self) -> PathBuf {
        let mut path = self.base_location.path.clone();

        for subfolder in self.get_our_subfolders() {
            path = path.join(subfolder);
        }

        path
    }

    pub fn get_file_path(&self, filename: &str) -> PathBuf {
        self.path().join(filename)
    }

    pub fn get_prefix_directories_longest_first(&self) -> Vec<PathBuf> {
        let mut directories = Vec::new();
        let mut current = self.base_location.path.clone();

        directories.push(current.clone());

        for subfolder in self.get_our_subfolders() {
            current = current.join(subfolder);
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
mod tests;
