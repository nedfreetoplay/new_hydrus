//! Quick download state from `hydrus/client/ClientDownloading.py`.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::{Value, json};

use crate::networking::client_networking::{HttpTransport, NetworkEngine};
use crate::networking::client_networking_jobs::NetworkJob;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GalleryIdentifier {
    pub site_type: i32,
    pub additional_info: String,
}

impl Default for GalleryIdentifier {
    fn default() -> Self {
        Self {
            site_type: 0,
            additional_info: String::new(),
        }
    }
}

impl GalleryIdentifier {
    pub fn new(site_type: i32, additional_info: impl Into<String>) -> Self {
        Self {
            site_type,
            additional_info: additional_info.into(),
        }
    }

    pub fn to_string(&self) -> String {
        "unknown downloader".to_string()
    }
}

impl SerialisableObject for GalleryIdentifier {
    fn serialisable_type(&self) -> u16 {
        hydrus_serialisable::SERIALISABLE_TYPE_GALLERY_IDENTIFIER
    }

    fn serialisable_version(&self) -> u16 {
        1
    }

    fn serialisable_name(&self) -> &'static str {
        "Gallery Identifier"
    }

    fn get_serialisable_info(&self) -> Value {
        json!([self.site_type, self.additional_info])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let values = info.as_array().ok_or_else(|| {
            hydrus_core::hydrus_exceptions::serialisation("GalleryIdentifier info must be a list")
        })?;

        self.site_type = values.first().and_then(Value::as_i64).unwrap_or(0) as i32;
        self.additional_info = values
            .get(1)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// `QuickDownloadManager` without the network layer: it remembers which hashes
/// are queued, active, finished or failed.
#[derive(Debug, Default)]
pub struct QuickDownloadManager {
    pending: BTreeSet<Vec<u8>>,
    active: BTreeSet<Vec<u8>>,
    finished: BTreeSet<Vec<u8>>,
    failed: BTreeSet<Vec<u8>>,
    urls: BTreeMap<Vec<u8>, String>,
}

impl QuickDownloadManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// `download_files( hashes )`
    pub fn download_files(&mut self, hashes: impl IntoIterator<Item = Vec<u8>>) {
        for hash in hashes {
            if self.finished.contains(&hash) || self.active.contains(&hash) {
                continue;
            }

            self.pending.insert(hash);
        }
    }

    pub fn pop_next(&mut self) -> Option<Vec<u8>> {
        let hash = self.pending.iter().next().cloned()?;
        self.pending.remove(&hash);
        self.active.insert(hash.clone());

        Some(hash)
    }

    pub fn finish(&mut self, hash: &[u8]) {
        if self.active.remove(hash) {
            self.finished.insert(hash.to_vec());
        }
    }

    pub fn fail(&mut self, hash: &[u8]) {
        if self.active.remove(hash) {
            self.failed.insert(hash.to_vec());
        }
    }

    pub fn is_queued(&self, hash: &[u8]) -> bool {
        self.pending.contains(hash)
    }

    /// Queues a download with the file URL used to fetch it.
    pub fn download_urls(&mut self, pairs: impl IntoIterator<Item = (Vec<u8>, String)>) {
        for (hash, url) in pairs {
            if self.finished.contains(&hash) || self.active.contains(&hash) {
                continue;
            }

            self.urls.insert(hash.clone(), url);
            self.pending.insert(hash);
        }
    }

    /// Runs one queued download and returns the downloaded bytes.
    pub fn run_next_download<T: HttpTransport>(
        &mut self,
        engine: &mut NetworkEngine<T>,
    ) -> Option<(Vec<u8>, HydrusResult<Vec<u8>>)> {
        let hash = self.pop_next()?;

        let Some(url) = self.urls.get(&hash).cloned() else {
            self.fail(&hash);
            return Some((
                hash,
                Err(hydrus_core::hydrus_exceptions::unknown(
                    "no URL was registered for this hash",
                )),
            ));
        };

        engine.add_job(NetworkJob::get(url));
        let job = engine.run_next()?;

        match job.get_response() {
            Some(body) => {
                self.finish(&hash);
                self.urls.remove(&hash);
                Some((hash, Ok(body.to_vec())))
            }
            None => {
                self.fail(&hash);
                let message = job.error.unwrap_or_else(|| "download failed".to_string());
                Some((hash, Err(hydrus_core::hydrus_exceptions::unknown(message))))
            }
        }
    }

    pub fn is_active(&self, hash: &[u8]) -> bool {
        self.active.contains(hash)
    }

    pub fn is_finished(&self, hash: &[u8]) -> bool {
        self.finished.contains(hash)
    }

    pub fn is_failed(&self, hash: &[u8]) -> bool {
        self.failed.contains(hash)
    }

    pub fn get_num_pending(&self) -> usize {
        self.pending.len()
    }

    pub fn get_num_active(&self) -> usize {
        self.active.len()
    }

    pub fn get_num_finished(&self) -> usize {
        self.finished.len()
    }

    pub fn get_num_failed(&self) -> usize {
        self.failed.len()
    }

    pub fn is_idle(&self) -> bool {
        self.pending.is_empty() && self.active.is_empty()
    }
}

#[cfg(test)]
mod tests;
