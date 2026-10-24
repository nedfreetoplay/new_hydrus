//! Noneable and list option defaults.

use serde_json::{Map, Value, json};

pub(super) fn noneable_integers() -> Map<String, Value> {
    let entries: &[(&str, Option<i64>)] = &[
        ("forced_search_limit", None),
        ("num_recent_tags", Some(20)),
        ("num_recent_tags_in_quick_dialog", Some(10)),
        ("duplicate_background_switch_intensity_a", Some(0)),
        ("duplicate_background_switch_intensity_b", Some(3)),
        ("duplicate_filter_auto_commit_batch_size", Some(1)),
        ("media_viewer_cursor_autohide_time_ms", Some(700)),
        ("animated_scanbar_hide_height", Some(5)),
        ("last_backup_time", None),
        ("export_path_character_limit", None),
        ("export_dirname_character_limit", None),
    ];

    entries
        .iter()
        .map(|(name, value)| (name.to_string(), json!(value)))
        .collect()
}

pub(super) fn noneable_strings() -> Map<String, Value> {
    let entries: &[(&str, Option<&str>)] = &[
        ("favourite_file_lookup_script", None),
        ("suggested_tags_layout", Some("notebook")),
        ("backup_path", None),
        ("last_png_export_dir", None),
        ("media_background_bmp_path", None),
        ("http_proxy", None),
        ("https_proxy", None),
        ("no_proxy", Some("127.0.0.1")),
        ("qt_style_name", None),
        ("qt_stylesheet_name", None),
    ];

    entries
        .iter()
        .map(|(name, value)| (name.to_string(), json!(value)))
        .collect()
}

pub(super) fn string_list() -> Map<String, Value> {
    let mut map = Map::new();

    map.insert(
        "default_media_viewer_custom_shortcuts".to_string(),
        json!([]),
    );
    map.insert("favourite_tags".to_string(), json!([]));
    map.insert(
        "advanced_file_deletion_reasons".to_string(),
        json!([
            "I do not like it.",
            "It is bad quality.",
            "It is not appropriate for this client.",
            "Temporary delete--I want to bring it back later."
        ]),
    );
    map.insert(
        "user_namespace_group_by_sort".to_string(),
        json!(["creator", "series", "character", "species", "", "meta"]),
    );

    map
}
