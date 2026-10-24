//! `ClientOptions._update_serialisable_info` migrations.

use hydrus_core::hydrus_exceptions::HydrusResult;
use serde_json::{Map, Value, json};

use crate::client_constants;

pub fn migrate(mut version: u16, mut info: Value) -> HydrusResult<(u16, Value)> {
    while version < super::SERIALISABLE_VERSION {
        info = migrate_one(version, info);
        version += 1;
    }

    Ok((version, info))
}

fn migrate_one(version: u16, mut info: Value) -> Value {
    let Some(dictionary) = info.as_object_mut() else {
        return info;
    };

    match version {
        1 => migrate_media_view_to_v2(dictionary),
        2 => migrate_portable_paths(dictionary),
        3 => migrate_media_view_to_v4(dictionary),
        4 => migrate_neighbouring_tags_to_routers(dictionary),
        5 => migrate_default_tag_sort(dictionary),
        6 => migrate_qmediaplayer_action(dictionary),
        7 => migrate_launch_paths(dictionary),
        _ => {}
    }

    info
}

/// v1 stored six values per mime; v2 drops the zoom flags back to a single
/// default zoom tuple.
fn migrate_media_view_to_v2(dictionary: &mut Map<String, Value>) {
    let Some(media_view) = dictionary.get_mut("media_view").and_then(Value::as_object_mut) else {
        return;
    };

    for (_, entry) in media_view.iter_mut() {
        let Some(values) = entry.as_array() else {
            continue;
        };

        if values.len() != 6 {
            continue;
        }

        *entry = json!([values[0].clone(), values[1].clone(), values[2].clone()]);
    }
}

fn migrate_portable_paths(dictionary: &mut Map<String, Value>) {
    let normalize = |value: &Value| -> Value {
        match value {
            Value::String(path) => Value::String(path.replace('\\', "/")),
            other => other.clone(),
        }
    };

    if let Some(weights) = dictionary
        .get_mut("client_files_locations_ideal_weights")
        .and_then(Value::as_array_mut)
    {
        for row in weights.iter_mut() {
            if let Some(pair) = row.as_array_mut() {
                if let Some(path) = pair.first_mut() {
                    *path = normalize(path);
                }
            }
        }
    }

    for key in [
        "client_files_locations_resized_thumbnail_override",
        "client_files_locations_full_size_thumbnail_override",
    ] {
        if let Some(value) = dictionary.get_mut(key) {
            *value = normalize(value);
        }
    }
}

/// v3 stored `(media_show, preview_show, zoom)`; v4 expands both show actions
/// into `(show, start_paused, start_with_embed)`.
fn migrate_media_view_to_v4(dictionary: &mut Map<String, Value>) {
    let Some(media_view) = dictionary
        .get_mut("media_view")
        .and_then(Value::as_object_mut)
    else {
        return;
    };

    for (_, entry) in media_view.iter_mut() {
        let Some(values) = entry.as_array() else {
            continue;
        };

        if values.len() != 3 {
            continue;
        }

        let media_show = values[0].clone();
        let preview_show = values[1].clone();
        let zoom = values[2].clone();

        *entry = json!([media_show, false, false, preview_show, false, false, zoom]);
    }
}

fn migrate_neighbouring_tags_to_routers(dictionary: &mut Map<String, Value>) {
    if let Some(key_list) = dictionary
        .get_mut("key_list")
        .and_then(Value::as_object_mut)
    {
        key_list.remove("default_neighbouring_txt_tag_service_keys");
    }

    dictionary
        .entry("default_export_files_metadata_routers")
        .or_insert_with(|| json!([]));
}

fn migrate_default_tag_sort(dictionary: &mut Map<String, Value>) {
    let Some(default_sort) = dictionary.remove("default_tag_sort") else {
        return;
    };

    let mut sorts = Map::new();

    for presentation in [
        "search_page",
        "search_page_manage_tags",
        "media_viewer",
        "media_viewer_manage_tags",
    ] {
        sorts.insert(presentation.to_string(), default_sort.clone());
    }

    dictionary.insert("default_tag_sorts".to_string(), Value::Object(sorts));
}

fn migrate_qmediaplayer_action(dictionary: &mut Map<String, Value>) {
    let Some(media_view) = dictionary
        .get_mut("media_view")
        .and_then(Value::as_object_mut)
    else {
        return;
    };

    for (_, entry) in media_view.iter_mut() {
        let Some(values) = entry.as_array_mut() else {
            continue;
        };

        for index in [0, 3] {
            if values.get(index).and_then(Value::as_i64)
                == Some(
                    client_constants::MEDIA_VIEWER_ACTION_SHOW_WITH_QMEDIAPLAYER_VIDEO_WIDGET
                        as i64,
                )
            {
                values[index] =
                    json!(client_constants::MEDIA_VIEWER_ACTION_SHOW_WITH_QTMEDIAPLAYER);
            }
        }
    }
}

fn migrate_launch_paths(dictionary: &mut Map<String, Value>) {
    let browser_path = dictionary
        .get_mut("noneable_strings")
        .and_then(Value::as_object_mut)
        .and_then(|strings| strings.remove("web_browser_path"));

    let browser_path = browser_path
        .and_then(|value| value.as_str().map(|path| path.replace("%path%", "%url%")))
        .map(Value::String)
        .unwrap_or(Value::Null);

    dictionary.insert(
        "web_browser_launch_paths".to_string(),
        json!([browser_path]),
    );

    if dictionary.remove("media_launch").is_some() {
        dictionary.insert("open_externally_launch_paths".to_string(), json!({}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_media_view_is_upgraded_through_all_versions() {
        let info = json!({
            "media_view": {
                "0": [0, 1, true, false, 3, 4]
            }
        });

        let (_, migrated) = migrate(1, info).unwrap();
        let entry = migrated["media_view"]["0"].as_array().unwrap();

        assert_eq!(entry.len(), 7);
        assert_eq!(entry[0], json!(0));
        assert_eq!(entry[3], json!(1));
    }

    #[test]
    fn v2_portable_paths_use_forward_slashes() {
        let info = json!({
            "client_files_locations_ideal_weights": [["C:\\hydrus\\files", 1]],
            "client_files_locations_full_size_thumbnail_override": "C:\\hydrus\\thumbs"
        });

        let (_, migrated) = migrate(2, info).unwrap();

        assert_eq!(
            migrated["client_files_locations_ideal_weights"][0][0],
            json!("C:/hydrus/files")
        );
        assert_eq!(
            migrated["client_files_locations_full_size_thumbnail_override"],
            json!("C:/hydrus/thumbs")
        );
    }

    #[test]
    fn old_versions_are_upgraded_to_current() {
        let (version, _) = migrate(5, json!({})).unwrap();

        assert_eq!(version, super::super::SERIALISABLE_VERSION);
    }

    #[test]
    fn v3_media_view_tuples_become_v4_seven_tuples() {
        let info = json!({
            "media_view": {
                "0": [0, 1, [1.0, 0, 0]]
            }
        });

        let (_, migrated) = migrate(3, info).unwrap();
        let entry = migrated["media_view"]["0"].as_array().unwrap();

        assert_eq!(entry.len(), 7);
        assert_eq!(entry[1], json!(false));
        assert_eq!(entry[2], json!(false));
    }

    #[test]
    fn v6_qmediaplayer_action_is_replaced() {
        let info = json!({
            "media_view": {
                "0": [8, false, false, 8, false, false, []]
            }
        });

        let (_, migrated) = migrate(6, info).unwrap();

        assert_eq!(migrated["media_view"]["0"][0], json!(9));
        assert_eq!(migrated["media_view"]["0"][3], json!(9));
    }

    #[test]
    fn v7_browser_path_moves_and_rewrites_placeholder() {
        let info = json!({
            "noneable_strings": { "web_browser_path": "firefox %path%" },
            "media_launch": { "0": "vlc" }
        });

        let (_, migrated) = migrate(7, info).unwrap();

        assert_eq!(
            migrated["web_browser_launch_paths"][0],
            json!("firefox %url%")
        );
        assert!(migrated.get("media_launch").is_none());
        assert!(migrated.get("open_externally_launch_paths").is_some());
    }
}
