//! Tag containers from `hydrus/client/metadata/ClientTags.py`.

use std::collections::{BTreeMap, BTreeSet};

use hydrus_core::hydrus_constants::ContentStatus;
use serde_json::{Value, json};

pub type TagStatus = ContentStatus;

/// `ServiceKeysToTags`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServiceKeysToTags {
    tags: BTreeMap<Vec<u8>, BTreeMap<String, TagStatus>>,
}

impl ServiceKeysToTags {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_status(
        &mut self,
        service_key: impl Into<Vec<u8>>,
        tag: impl Into<String>,
        status: TagStatus,
    ) {
        self.tags
            .entry(service_key.into())
            .or_default()
            .insert(tag.into(), status);
    }

    pub fn add_tags(
        &mut self,
        service_key: impl Into<Vec<u8>>,
        tags: impl IntoIterator<Item = impl Into<String>>,
        status: TagStatus,
    ) {
        let service_key = service_key.into();

        for tag in tags {
            self.add_status(service_key.clone(), tag, status);
        }
    }

    pub fn get_status(&self, service_key: &[u8], tag: &str) -> Option<TagStatus> {
        self.tags
            .get(service_key)
            .and_then(|tags| tags.get(tag))
            .copied()
    }

    pub fn get_tags(&self, service_key: &[u8]) -> BTreeSet<&str> {
        self.tags
            .get(service_key)
            .map(|tags| tags.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    pub fn get_tags_with_status(&self, service_key: &[u8], status: TagStatus) -> BTreeSet<&str> {
        self.tags
            .get(service_key)
            .map(|tags| {
                tags.iter()
                    .filter(|(_, tag_status)| **tag_status == status)
                    .map(|(tag, _)| tag.as_str())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_service_keys(&self) -> Vec<&[u8]> {
        self.tags.keys().map(Vec::as_slice).collect()
    }

    pub fn remove_tag(&mut self, service_key: &[u8], tag: &str) {
        let mut remove_service = false;

        if let Some(tags) = self.tags.get_mut(service_key) {
            tags.remove(tag);
            remove_service = tags.is_empty();
        }

        if remove_service {
            self.tags.remove(service_key);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tags.values().all(BTreeMap::is_empty)
    }

    pub fn duplicate(&self) -> Self {
        self.clone()
    }

    /// `_get_serialisable_info`
    pub fn get_serialisable_info(&self) -> Value {
        let serialised: Vec<Value> = self
            .tags
            .iter()
            .map(|(service_key, tags)| {
                let tags: Vec<Value> = tags
                    .iter()
                    .map(|(tag, status)| json!([tag, status.as_int()]))
                    .collect();

                json!([hex_encode(service_key), tags])
            })
            .collect();

        Value::Array(serialised)
    }

    pub fn initialise_from_serialisable_info(&mut self, info: &Value) -> Option<()> {
        let services = info.as_array()?;
        self.tags.clear();

        for service in services {
            let service = service.as_array()?;
            let service_key = hex_decode(service.first()?.as_str()?)?;
            let tags = service.get(1)?.as_array()?;

            for tag in tags {
                let tag = tag.as_array()?;
                let name = tag.first()?.as_str()?.to_string();
                let status = TagStatus::try_from_int(tag.get(1)?.as_u64()? as u8)?;
                self.tags
                    .entry(service_key.clone())
                    .or_default()
                    .insert(name, status);
            }
        }

        Some(())
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests;
