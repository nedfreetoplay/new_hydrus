//! Client API backend backed by the real client database.
//!
//! The Python client exposes a small HTTP API through its local server service.
//! This backend implements the read/search endpoints and tag writes used by the
//! Rust port while keeping transport concerns in `api::server`.

use std::collections::{BTreeMap, BTreeSet};

use hydrus_core::hydrus_constants::{ContentStatus, ServiceType};
use hydrus_core::hydrus_db_base::DBBase;
use serde_json::{Value, json};

use crate::api::manager::{ApiBackend, ApiServiceInfo};
use crate::db::{
    files_inbox, files_metadata, files_storage, mappings, master, master_tags, services,
    tag_search, url_map,
};
use crate::files::ClientFilesManager;

#[derive(Clone)]
pub struct ClientApiBackend {
    db: DBBase,
    files_manager: ClientFilesManager,
    services: Vec<services::Service>,
}

impl ClientApiBackend {
    pub fn new(
        db: DBBase,
        files_manager: ClientFilesManager,
        services: Vec<services::Service>,
    ) -> Self {
        Self {
            db,
            files_manager,
            services,
        }
    }

    fn local_tag_service_id(&self) -> Option<i64> {
        self.services
            .iter()
            .find(|service| service.name == "local tags")
            .or_else(|| {
                self.services
                    .iter()
                    .find(|service| service.service_type == ServiceType::LocalTag)
            })
            .map(|service| service.service_id)
    }

    fn local_file_service_ids(&self) -> Vec<i64> {
        let mut ids: Vec<i64> = self
            .services
            .iter()
            .filter(|service| service.service_type == ServiceType::LocalFileDomain)
            .map(|service| service.service_id)
            .collect();

        if ids.is_empty() {
            if let Some(service) = self
                .services
                .iter()
                .find(|service| service.name == "my files")
            {
                ids.push(service.service_id);
            }
        }

        ids
    }

    fn all_local_hash_ids(&self) -> BTreeSet<i64> {
        let mut ids = BTreeSet::new();

        for service_id in self.local_file_service_ids() {
            if let Ok(current) =
                files_storage::get_hash_ids(&self.db, service_id, ContentStatus::Current)
            {
                ids.extend(current);
            }
        }

        ids
    }

    fn metadata(&self, file_id: i64) -> Option<Value> {
        let hash = master::get_hash(&self.db, file_id).ok().flatten()?;
        let info = files_metadata::get_file_info(&self.db, file_id)
            .ok()
            .flatten()?;

        let mut tags: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for service in &self.services {
            if !matches!(service.service_type, ServiceType::LocalTag) {
                continue;
            }

            if let Ok(values) = mappings::get_tags(
                &self.db,
                service.service_id,
                ContentStatus::Current,
                file_id,
            ) {
                if !values.is_empty() {
                    tags.insert(service.name.clone(), values);
                }
            }
        }

        let urls = url_map::get_urls(&self.db, file_id).unwrap_or_default();

        let file_services: Vec<String> = self
            .services
            .iter()
            .filter(|service| matches!(service.service_type, ServiceType::LocalFileDomain))
            .filter(|service| {
                files_storage::get_hash_ids(&self.db, service.service_id, ContentStatus::Current)
                    .map(|ids| ids.contains(&file_id))
                    .unwrap_or(false)
            })
            .map(|service| service.name.clone())
            .collect();

        Some(json!({
            "file_id": file_id,
            "hash": hex(&hash),
            "size": info.size,
            "mime": info.mime.mimetype_str(),
            "mime_type": info.mime.as_int(),
            "width": info.width,
            "height": info.height,
            "duration": info.duration_ms,
            "num_frames": info.num_frames,
            "has_audio": info.has_audio,
            "is_inbox": files_inbox::is_inbox(&self.db, file_id).unwrap_or(false),
            "tags": tags,
            "urls": urls,
            "file_services": file_services,
        }))
    }
}

impl ApiBackend for ClientApiBackend {
    fn get_services(&self) -> Vec<ApiServiceInfo> {
        self.services
            .iter()
            .map(|service| ApiServiceInfo {
                name: service.name.clone(),
                service_key: hex(&service.service_key),
                service_type: service.service_type.as_str().to_string(),
            })
            .collect()
    }

    fn search_files(&self, tags: &[String]) -> Vec<i64> {
        let mut result = self.all_local_hash_ids();

        let inbox: BTreeSet<i64> = files_inbox::get_inbox_hash_ids(&self.db)
            .unwrap_or_default()
            .into_iter()
            .collect();

        let mut limit: Option<usize> = None;
        let Some(tag_service_id) = self.local_tag_service_id() else {
            return Vec::new();
        };

        for term in tags {
            let term = term.trim();

            if term.is_empty() {
                continue;
            }

            let negative = term.starts_with('-') && term.len() > 1;
            let clean = if negative { &term[1..] } else { term };
            let lower = clean.to_ascii_lowercase();

            if let Some(value) = lower.strip_prefix("system:limit=") {
                limit = value.parse::<usize>().ok();
                continue;
            }

            let term_set: Option<BTreeSet<i64>> = if lower == "system:inbox" {
                Some(inbox.clone())
            } else if lower == "system:archive" {
                Some(result.difference(&inbox).copied().collect())
            } else if lower == "system:everything" || lower == "system:all" {
                Some(result.clone())
            } else if lower.starts_with("system:") {
                None
            } else if let Some(hash) = lower.strip_prefix("system:hash=") {
                let hash = unhex(hash).unwrap_or_default();
                master::find_hash_id(&self.db, &hash)
                    .ok()
                    .flatten()
                    .map(|id| BTreeSet::from([id]))
            } else if clean.ends_with(':') {
                tag_search::get_hash_ids_for_namespace(
                    &self.db,
                    tag_service_id,
                    ContentStatus::Current,
                    clean.trim_end_matches(':'),
                )
                .ok()
            } else if clean.contains('*') {
                tag_search::get_hash_ids_for_wildcard(
                    &self.db,
                    tag_service_id,
                    ContentStatus::Current,
                    clean,
                )
                .ok()
            } else {
                tag_search::get_hash_ids_for_tag(
                    &self.db,
                    tag_service_id,
                    ContentStatus::Current,
                    clean,
                )
                .ok()
            };

            let Some(term_set) = term_set else {
                continue;
            };

            if negative {
                result.retain(|id| !term_set.contains(id));
            } else {
                result.retain(|id| term_set.contains(id));
            }

            if result.is_empty() {
                break;
            }
        }

        let mut ids: Vec<i64> = result.into_iter().collect();
        ids.sort_unstable();

        if let Some(limit) = limit {
            ids.truncate(limit);
        }

        ids
    }

    fn file_metadata(&self, file_ids: &[i64]) -> Vec<Value> {
        file_ids
            .iter()
            .filter_map(|file_id| self.metadata(*file_id))
            .collect()
    }

    fn file_binary(&self, file_id: i64) -> Option<(String, Vec<u8>)> {
        let info = files_metadata::get_file_info(&self.db, file_id)
            .ok()
            .flatten()?;
        let hash = master::get_hash(&self.db, file_id).ok().flatten()?;
        let path = self
            .files_manager
            .get_file_path(&hash, info.mime, false)
            .ok()?;
        let bytes = std::fs::read(path).ok()?;

        Some((info.mime.mimetype_str().to_string(), bytes))
    }

    fn file_ids_for_hashes(&self, hashes: &[String]) -> Vec<i64> {
        hashes
            .iter()
            .filter_map(|hash| unhex(hash))
            .filter_map(|hash| master::find_hash_id(&self.db, &hash).ok().flatten())
            .collect()
    }

    fn add_tags(
        &self,
        service_name: &str,
        file_ids: &[i64],
        tags: &[String],
    ) -> Result<usize, String> {
        let service = self
            .services
            .iter()
            .find(|service| {
                service.name == service_name && service.service_type == ServiceType::LocalTag
            })
            .ok_or_else(|| format!("unknown local tag service: {service_name}"))?;

        let mut tag_ids = Vec::new();

        for tag in tags {
            let tag = tag.trim();

            if !tag.is_empty() {
                tag_ids.push(
                    master_tags::get_tag_id(&self.db, tag).map_err(|error| error.to_string())?,
                );
            }
        }

        let mut changed = 0;

        for file_id in file_ids {
            mappings::add_mappings(
                &self.db,
                service.service_id,
                ContentStatus::Current,
                *file_id,
                &tag_ids,
            )
            .map_err(|error| error.to_string())?;
            changed += tag_ids.len();
        }

        Ok(changed)
    }

    fn remove_tags(
        &self,
        service_name: &str,
        file_ids: &[i64],
        tags: &[String],
    ) -> Result<usize, String> {
        let service = self
            .services
            .iter()
            .find(|service| {
                service.name == service_name && service.service_type == ServiceType::LocalTag
            })
            .ok_or_else(|| format!("unknown local tag service: {service_name}"))?;

        let mut tag_ids = Vec::new();

        for tag in tags {
            let tag = tag.trim();

            if !tag.is_empty() {
                if let Ok(tag_id) = master_tags::get_tag_id(&self.db, tag) {
                    tag_ids.push(tag_id);
                }
            }
        }

        let mut changed = 0;

        for file_id in file_ids {
            mappings::remove_mappings(
                &self.db,
                service.service_id,
                ContentStatus::Current,
                *file_id,
                &tag_ids,
            )
            .map_err(|error| error.to_string())?;
            changed += tag_ids.len();
        }

        Ok(changed)
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(value: &str) -> Option<Vec<u8>> {
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
