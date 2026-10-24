//! Lazy managers attached to a media result.

use std::collections::BTreeMap;

use hydrus_core::files::hydrus_file_handling::FileInfo;

use crate::metadata::{RatingsManager, ServiceKeysToTags};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfoManager {
    pub hash: Vec<u8>,
    pub file_info: FileInfo,
}

impl FileInfoManager {
    pub fn new(hash: Vec<u8>, file_info: FileInfo) -> Self {
        Self { hash, file_info }
    }

    pub fn get_hash(&self) -> &[u8] {
        &self.hash
    }

    pub fn get_size(&self) -> u64 {
        self.file_info.size
    }

    pub fn get_mime(&self) -> hydrus_core::hydrus_constants::Mime {
        self.file_info.mime
    }

    pub fn get_resolution(&self) -> Option<(u32, u32)> {
        match (self.file_info.width, self.file_info.height) {
            (Some(width), Some(height)) => Some((width, height)),
            _ => None,
        }
    }

    pub fn has_duration(&self) -> bool {
        self.file_info.duration_ms.is_some()
    }

    pub fn has_audio(&self) -> bool {
        self.file_info.has_audio
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimesManager {
    pub times: BTreeMap<i32, Option<i64>>,
}

impl TimesManager {
    pub fn get_time(&self, timestamp_type: i32) -> Option<i64> {
        self.times.get(&timestamp_type).copied().flatten()
    }

    pub fn set_time(&mut self, timestamp_type: i32, timestamp: Option<i64>) {
        self.times.insert(timestamp_type, timestamp);
    }

    pub fn has_time(&self, timestamp_type: i32) -> bool {
        self.get_time(timestamp_type).is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocationsManager {
    current: BTreeMap<Vec<u8>, i64>,
    deleted: BTreeMap<Vec<u8>, i64>,
    pub inbox: bool,
}

impl LocationsManager {
    pub fn new(inbox: bool) -> Self {
        Self {
            inbox,
            ..Default::default()
        }
    }

    pub fn set_current(&mut self, service_key: impl Into<Vec<u8>>, timestamp: i64) {
        self.current.insert(service_key.into(), timestamp);
    }

    pub fn set_deleted(&mut self, service_key: impl Into<Vec<u8>>, timestamp: i64) {
        self.deleted.insert(service_key.into(), timestamp);
    }

    pub fn is_current(&self, service_key: &[u8]) -> bool {
        self.current.contains_key(service_key)
    }

    pub fn is_deleted(&self, service_key: &[u8]) -> bool {
        self.deleted.contains_key(service_key)
    }

    pub fn has_archive(&self) -> bool {
        !self.inbox
    }

    pub fn current(&self) -> &BTreeMap<Vec<u8>, i64> {
        &self.current
    }

    pub fn deleted(&self) -> &BTreeMap<Vec<u8>, i64> {
        &self.deleted
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotesManager {
    notes: BTreeMap<String, String>,
}

impl NotesManager {
    pub fn set_note(&mut self, name: impl Into<String>, note: impl Into<String>) {
        self.notes.insert(name.into(), note.into());
    }

    pub fn get_note(&self, name: &str) -> Option<&str> {
        self.notes.get(name).map(String::as_str)
    }

    pub fn delete_note(&mut self, name: &str) {
        self.notes.remove(name);
    }

    pub fn has_notes(&self) -> bool {
        !self.notes.is_empty()
    }

    pub fn notes(&self) -> &BTreeMap<String, String> {
        &self.notes
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TagsManager {
    pub tags: ServiceKeysToTags,
}

impl TagsManager {
    pub fn get_tags(&self, service_key: &[u8]) -> Vec<String> {
        self.tags
            .get_tags(service_key)
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    pub fn has_tags(&self, service_key: &[u8]) -> bool {
        !self.tags.get_tags(service_key).is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileViewingStatsManager {
    pub views: i64,
    pub viewtime_ms: i64,
    pub last_viewed_timestamp_ms: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MediaManagers {
    pub ratings: RatingsManager,
    pub tags: TagsManager,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydrus_core::hydrus_constants::Mime;

    fn file_info() -> FileInfo {
        FileInfo {
            size: 1024,
            mime: Mime::Png,
            width: Some(100),
            height: Some(200),
            duration_ms: None,
            num_frames: None,
            has_audio: false,
            num_words: None,
        }
    }

    #[test]
    fn file_info_manager_exposes_basic_metadata() {
        let manager = FileInfoManager::new(vec![1, 2], file_info());

        assert_eq!(manager.get_size(), 1024);
        assert_eq!(manager.get_resolution(), Some((100, 200)));
        assert!(!manager.has_audio());
    }

    #[test]
    fn locations_track_inbox_and_services() {
        let mut manager = LocationsManager::new(true);
        manager.set_current(b"my files", 100);

        assert!(manager.is_current(b"my files"));
        assert!(!manager.has_archive());
    }

    #[test]
    fn notes_are_named_entries() {
        let mut manager = NotesManager::default();
        manager.set_note("page 1", "hello");

        assert!(manager.has_notes());
        assert_eq!(manager.get_note("page 1"), Some("hello"));
    }
}
