//! Trash maintenance from `hydrus/client/files/ClientTrashManager.py`.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashedFile {
    pub hash: Vec<u8>,
    pub timestamp: i64,
    pub size: u64,
}

#[derive(Debug, Clone, Default)]
pub struct TrashManager {
    files: BTreeMap<Vec<u8>, TrashedFile>,
    pub max_age_hours: i64,
    pub max_size_mb: i64,
}

impl TrashManager {
    pub fn new(max_age_hours: i64, max_size_mb: i64) -> Self {
        Self {
            files: BTreeMap::new(),
            max_age_hours,
            max_size_mb,
        }
    }

    pub fn add_file(&mut self, file: TrashedFile) {
        self.files.insert(file.hash.clone(), file);
    }

    pub fn remove_file(&mut self, hash: &[u8]) -> Option<TrashedFile> {
        self.files.remove(hash)
    }

    pub fn get_file(&self, hash: &[u8]) -> Option<&TrashedFile> {
        self.files.get(hash)
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn total_size(&self) -> u64 {
        self.files.values().map(|file| file.size).sum()
    }

    pub fn get_expired(&self, now: i64) -> Vec<&TrashedFile> {
        let max_age_seconds = self.max_age_hours * 3_600;

        self.files
            .values()
            .filter(|file| now - file.timestamp > max_age_seconds)
            .collect()
    }

    /// Returns the oldest files whose removal brings the trash under its size
    /// budget. The limit is in megabytes, as in the legacy options.
    pub fn get_over_size(&self) -> Vec<&TrashedFile> {
        let max_bytes = self.max_size_mb.max(0) as u64 * 1024 * 1024;

        if self.total_size() <= max_bytes {
            return Vec::new();
        }

        let mut files: Vec<&TrashedFile> = self.files.values().collect();
        files.sort_by_key(|file| file.timestamp);

        let mut remaining = self.total_size();
        let mut removed = Vec::new();

        for file in files {
            if remaining <= max_bytes {
                break;
            }

            removed.push(file);
            remaining = remaining.saturating_sub(file.size);
        }

        removed
    }

    pub fn get_files_to_delete(&self, now: i64) -> Vec<Vec<u8>> {
        let mut hashes: Vec<Vec<u8>> = self
            .get_expired(now)
            .into_iter()
            .map(|file| file.hash.clone())
            .collect();

        for file in self.get_over_size() {
            if !hashes.contains(&file.hash) {
                hashes.push(file.hash.clone());
            }
        }

        hashes
    }

    pub fn clear(&mut self) {
        self.files.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(hash: u8, timestamp: i64, size: u64) -> TrashedFile {
        TrashedFile {
            hash: vec![hash],
            timestamp,
            size,
        }
    }

    #[test]
    fn expired_files_respect_max_age() {
        let mut manager = TrashManager::new(72, 2048);
        manager.add_file(file(1, 0, 10));
        manager.add_file(file(2, 1_000_000, 10));

        let expired = manager.get_expired(72 * 3_600 + 1);

        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].hash, vec![1]);
    }

    #[test]
    fn over_size_removes_oldest_first() {
        let mut manager = TrashManager::new(72, 1);
        manager.add_file(file(1, 10, 600 * 1024));
        manager.add_file(file(2, 20, 600 * 1024));
        manager.add_file(file(3, 30, 600 * 1024));

        let removed = manager.get_over_size();

        assert_eq!(removed.len(), 2);
        assert_eq!(removed[0].hash, vec![1]);
        assert_eq!(removed[1].hash, vec![2]);
    }

    #[test]
    fn deletion_list_combines_age_and_size() {
        let mut manager = TrashManager::new(1, 1);
        manager.add_file(file(1, 0, 100));
        manager.add_file(file(2, 2 * 3_600, 2 * 1024 * 1024));

        let hashes = manager.get_files_to_delete(2 * 3_600);

        assert!(hashes.contains(&vec![1]));
        assert!(hashes.contains(&vec![2]));
    }

    #[test]
    fn removing_and_clearing_work() {
        let mut manager = TrashManager::new(1, 1);
        manager.add_file(file(1, 0, 1));

        assert_eq!(manager.remove_file(&[1]).unwrap().size, 1);
        assert!(manager.is_empty());

        manager.add_file(file(2, 0, 1));
        manager.clear();
        assert!(manager.is_empty());
    }
}
