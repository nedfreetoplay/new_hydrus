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
mod tests;
