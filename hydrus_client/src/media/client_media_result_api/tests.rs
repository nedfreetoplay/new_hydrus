
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
