//! Content updates from `hydrus/client/metadata/ClientContentUpdates.py`.

use hydrus_core::hydrus_constants::{ContentStatus, ContentType};
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentAction {
    Add,
    Delete,
    Pend,
    Rescind,
    Petition,
    RescindPetition,
    Set,
    Manage,
}

impl ContentAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Delete => "delete",
            Self::Pend => "pend",
            Self::Rescind => "rescind",
            Self::Petition => "petition",
            Self::RescindPetition => "rescind petition",
            Self::Set => "set",
            Self::Manage => "manage",
        }
    }
}

/// One update row. Python has a class per content type; the Rust port keeps a
/// shared shape with typed payload fields so it can be persisted generically.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentUpdate {
    pub action: ContentAction,
    pub service_key: Vec<u8>,
    pub content_type: ContentType,
    pub hashes: Vec<Vec<u8>>,
    pub tags: Vec<String>,
    pub urls: Vec<String>,
    pub details: Map<String, Value>,
}

impl ContentUpdate {
    pub fn new(action: ContentAction, service_key: Vec<u8>, content_type: ContentType) -> Self {
        Self {
            action,
            service_key,
            content_type,
            hashes: Vec::new(),
            tags: Vec::new(),
            urls: Vec::new(),
            details: Map::new(),
        }
    }

    pub fn with_hashes(mut self, hashes: impl IntoIterator<Item = Vec<u8>>) -> Self {
        self.hashes = hashes.into_iter().collect();
        self
    }

    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_urls(mut self, urls: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.urls = urls.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_detail(mut self, key: &str, value: Value) -> Self {
        self.details.insert(key.to_string(), value);
        self
    }

    pub fn status_for_action(&self) -> Option<ContentStatus> {
        match self.action {
            ContentAction::Add | ContentAction::Set => Some(ContentStatus::Current),
            ContentAction::Pend => Some(ContentStatus::Pending),
            ContentAction::Petition => Some(ContentStatus::Petitioned),
            ContentAction::Delete => Some(ContentStatus::Deleted),
            ContentAction::Rescind | ContentAction::RescindPetition | ContentAction::Manage => None,
        }
    }

    pub fn to_dict_for_api(&self) -> Value {
        json!({
            "action": self.action.as_str(),
            "service_key": hex_encode(&self.service_key),
            "content_type": self.content_type.as_int(),
            "hashes": self.hashes.iter().map(hex_encode).collect::<Vec<_>>(),
            "tags": self.tags,
            "urls": self.urls,
            "details": Value::Object(self.details.clone()),
        })
    }
}

/// `ContentUpdatePackage`
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContentUpdatePackage {
    pub updates: Vec<ContentUpdate>,
}

impl ContentUpdatePackage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_updates(updates: Vec<ContentUpdate>) -> Self {
        Self { updates }
    }

    pub fn add_update(&mut self, update: ContentUpdate) {
        self.updates.push(update);
    }

    pub fn is_empty(&self) -> bool {
        self.updates.is_empty()
    }

    pub fn extend(&mut self, other: ContentUpdatePackage) {
        self.updates.extend(other.updates);
    }

    pub fn get_mappings(&self, service_key: &[u8]) -> Vec<&ContentUpdate> {
        self.updates
            .iter()
            .filter(|update| {
                update.content_type == ContentType::Mappings && update.service_key == service_key
            })
            .collect()
    }

    pub fn get_files(&self, service_key: &[u8]) -> Vec<&ContentUpdate> {
        self.updates
            .iter()
            .filter(|update| {
                update.content_type == ContentType::Files && update.service_key == service_key
            })
            .collect()
    }

    pub fn to_dict_for_api(&self) -> Value {
        Value::Array(
            self.updates
                .iter()
                .map(ContentUpdate::to_dict_for_api)
                .collect(),
        )
    }
}

fn hex_encode(bytes: &Vec<u8>) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests;
