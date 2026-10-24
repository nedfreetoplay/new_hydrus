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
mod tests {
    use super::*;
    use hydrus_core::files::hydrus_file_handling::FileInfo;
    use hydrus_core::hydrus_constants::Mime;

    fn result() -> MediaResult {
        MediaResult::new(
            vec![0xab, 0xcd],
            FileInfo {
                size: 100,
                mime: Mime::Png,
                width: Some(10),
                height: Some(20),
                duration_ms: None,
                num_frames: None,
                has_audio: false,
                num_words: None,
            },
        )
    }

    #[test]
    fn simple_api_dict_has_core_fields() {
        let dict = media_result_to_dict(&result(), true);

        assert_eq!(dict["file_id"], json!("abcd"));
        assert_eq!(dict["size"], json!(100));
        assert_eq!(dict["ext"], json!("png"));
        assert!(dict.get("tags").is_none());
    }

    #[test]
    fn full_api_dict_includes_managers() {
        let mut media = result();
        media.tags_manager.tags.add_status(
            b"local tags",
            "blue eyes",
            crate::metadata::TagStatus::Current,
        );

        let dict = media_result_to_dict(&media, false);

        assert_eq!(dict["tags"]["6c6f63616c2074616773"][0], json!("blue eyes"));
        assert!(dict.get("locations").is_some());
    }

    #[test]
    fn cache_is_keyed_by_hash() {
        let mut cache = MediaResultCache::new();
        let media = Arc::new(result());

        cache.add_media_result(media.clone());

        assert!(cache.contains(&[0xab, 0xcd]));
        assert_eq!(cache.get(&[0xab, 0xcd]).unwrap().get_size(), 100);

        cache.clear();

        assert!(cache.is_empty());
    }

    #[test]
    fn ratings_render_for_the_api() {
        assert_eq!(rating_to_api_value(Rating::Like), json!("like"));
        assert_eq!(rating_to_api_value(Rating::Numerical(0.5)), json!(0.5));
    }
}
