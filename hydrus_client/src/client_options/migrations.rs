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
mod tests;
