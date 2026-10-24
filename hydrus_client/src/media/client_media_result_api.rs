//! Media result API serialisation and caching (`ClientMediaResultAPI.py`).

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};

use crate::media::MediaResult;
use crate::metadata::Rating;

pub fn media_result_to_dict(media_result: &MediaResult, simple: bool) -> Value {
    let hash = hex_encode(media_result.get_hash());
    let mime = media_result.get_mime();

    let mut dict = serde_json::Map::new();

    dict.insert("file_id".to_string(), json!(hash));
    dict.insert("size".to_string(), json!(media_result.get_size()));
    dict.insert("mime".to_string(), json!(mime.as_int()));
    dict.insert(
        "ext".to_string(),
        json!(mime.extension().trim_start_matches('.')),
    );
    dict.insert("has_audio".to_string(), json!(media_result.has_audio()));
    dict.insert("is_inbox".to_string(), json!(media_result.get_inbox()));

    if let Some((width, height)) = media_result.get_resolution() {
        dict.insert("width".to_string(), json!(width));
        dict.insert("height".to_string(), json!(height));
    }

    if let Some(duration) = media_result.get_duration_ms() {
        dict.insert("duration".to_string(), json!(duration));
    }

    if let Some(frames) = media_result.get_num_frames() {
        dict.insert("num_frames".to_string(), json!(frames));
    }

    if let Some(words) = media_result.get_num_words() {
        dict.insert("num_words".to_string(), json!(words));
    }

    if !simple {
        let tags: BTreeMap<String, Vec<String>> = media_result
            .tags_manager
            .tags
            .get_service_keys()
            .into_iter()
            .map(|service_key| {
                (
                    hex_encode(service_key),
                    media_result.tags_manager.get_tags(service_key),
                )
            })
            .filter(|(_, tags)| !tags.is_empty())
            .collect();

        dict.insert("tags".to_string(), json!(tags));

        let notes: BTreeMap<String, String> = media_result
            .notes_manager
            .notes()
            .iter()
            .map(|(name, note)| (name.clone(), note.clone()))
            .collect();

        dict.insert("notes".to_string(), json!(notes));

        let ratings: BTreeMap<String, f64> = media_result
            .ratings_manager
            .ratings()
            .iter()
            .map(|(service_key, rating)| (hex_encode(service_key), rating.value()))
            .collect();

        dict.insert("ratings".to_string(), json!(ratings));

        dict.insert(
            "locations".to_string(),
            json!({
                "current": media_result.locations_manager.current().keys().map(|key| hex_encode(key)).collect::<Vec<_>>(),
                "deleted": media_result.locations_manager.deleted().keys().map(|key| hex_encode(key)).collect::<Vec<_>>(),
            }),
        );
    }

    Value::Object(dict)
}

pub fn rating_to_api_value(rating: Rating) -> Value {
    match rating {
        Rating::Like => json!("like"),
        Rating::Dislike => json!("dislike"),
        Rating::Numerical(value) => json!(value),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, Clone, Default)]
pub struct MediaResultCache {
    results: BTreeMap<Vec<u8>, Arc<MediaResult>>,
}

impl MediaResultCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_media_result(&mut self, media_result: Arc<MediaResult>) {
        self.results
            .insert(media_result.get_hash().to_vec(), media_result);
    }

    pub fn get(&self, hash: &[u8]) -> Option<&Arc<MediaResult>> {
        self.results.get(hash)
    }

    pub fn contains(&self, hash: &[u8]) -> bool {
        self.results.contains_key(hash)
    }

    pub fn remove(&mut self, hash: &[u8]) -> Option<Arc<MediaResult>> {
        self.results.remove(hash)
    }

    pub fn clear(&mut self) {
        self.results.clear();
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn all(&self) -> impl Iterator<Item = &Arc<MediaResult>> {
        self.results.values()
    }
}

#[cfg(test)]
mod tests;
