//! File seeds and their cache from
//! `hydrus/client/importing/ClientImportFileSeeds.py`.

use std::collections::BTreeMap;

use crate::client_constants;

pub const FILE_SEED_TYPE_URL: i32 = 0;
pub const FILE_SEED_TYPE_HDD: i32 = 1;
pub const FILE_SEED_TYPE_IPFS: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSeed {
    pub file_seed_type: i32,
    pub file_seed_data: String,
    pub status: i32,
    pub note: String,
    pub created: i64,
    pub modified: i64,
    pub hash: Option<Vec<u8>>,
}

impl FileSeed {
    pub fn url(url: impl Into<String>, created: i64) -> Self {
        Self {
            file_seed_type: FILE_SEED_TYPE_URL,
            file_seed_data: url.into(),
            status: client_constants::STATUS_UNKNOWN,
            note: String::new(),
            created,
            modified: created,
            hash: None,
        }
    }

    pub fn hdd(path: impl Into<String>, created: i64) -> Self {
        Self {
            file_seed_type: FILE_SEED_TYPE_HDD,
            file_seed_data: path.into(),
            ..Self::url(String::new(), created)
        }
    }

    pub fn get_status(&self) -> i32 {
        self.status
    }

    pub fn set_status(&mut self, status: i32, modified: i64) {
        self.status = status;
        self.modified = modified;
    }

    pub fn set_hash(&mut self, hash: Vec<u8>, modified: i64) {
        self.hash = Some(hash);
        self.modified = modified;
    }

    pub fn set_note(&mut self, note: impl Into<String>, modified: i64) {
        self.note = note.into();
        self.modified = modified;
    }

    pub fn is_done(&self) -> bool {
        !matches!(
            self.status,
            client_constants::STATUS_UNKNOWN | client_constants::STATUS_NEW
        )
    }

    pub fn to_string(&self) -> String {
        let status = match self.status {
            client_constants::STATUS_UNKNOWN => "unknown",
            client_constants::STATUS_SUCCESSFUL_AND_NEW => "successful and new",
            client_constants::STATUS_SUCCESSFUL_BUT_REDUNDANT => "successful but redundant",
            client_constants::STATUS_DELETED => "deleted",
            client_constants::STATUS_ERROR => "error",
            client_constants::STATUS_NEW => "new",
            client_constants::STATUS_PAUSED => "paused",
            _ => "unknown",
        };

        format!("{status}: {}", self.file_seed_data)
    }
}

#[derive(Debug, Default)]
pub struct FileSeedCache {
    seeds: BTreeMap<String, FileSeed>,
}

impl FileSeedCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_seed(&mut self, seed: FileSeed) -> bool {
        if self.seeds.contains_key(&seed.file_seed_data) {
            return false;
        }

        self.seeds.insert(seed.file_seed_data.clone(), seed);
        true
    }

    pub fn add_seeds(&mut self, seeds: impl IntoIterator<Item = FileSeed>) -> usize {
        seeds
            .into_iter()
            .filter(|seed| self.add_seed(seed.clone()))
            .count()
    }

    pub fn get_seed(&self, key: &str) -> Option<&FileSeed> {
        self.seeds.get(key)
    }

    pub fn get_seed_mut(&mut self, key: &str) -> Option<&mut FileSeed> {
        self.seeds.get_mut(key)
    }

    pub fn remove_seed(&mut self, key: &str) -> Option<FileSeed> {
        self.seeds.remove(key)
    }

    pub fn set_status(&mut self, key: &str, status: i32, modified: i64) -> bool {
        if let Some(seed) = self.seeds.get_mut(key) {
            seed.set_status(status, modified);
            true
        } else {
            false
        }
    }

    pub fn get_seeds_with_status(&self, status: i32) -> Vec<&FileSeed> {
        self.seeds
            .values()
            .filter(|seed| seed.status == status)
            .collect()
    }

    pub fn get_undone_seeds(&self) -> Vec<&FileSeed> {
        self.seeds.values().filter(|seed| !seed.is_done()).collect()
    }

    pub fn get_num_seeds(&self) -> usize {
        self.seeds.len()
    }

    pub fn get_num_done(&self) -> usize {
        self.seeds.values().filter(|seed| seed.is_done()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.seeds.is_empty()
    }

    pub fn clear(&mut self) {
        self.seeds.clear();
    }
}

#[cfg(test)]
mod tests;
