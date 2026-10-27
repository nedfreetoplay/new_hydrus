//! Per-file tag, rating and note panels.
//!
//! Hydrus shows these three panels together for the current selection. The
//! rendering lives here and the Qt layer just draws the dictionaries; all
//! validation and database writes stay in this module so both the query page,
//! the media viewer and the manage-tags dialog share one implementation.

use std::collections::BTreeMap;

use hydrus_core::hydrus_constants::{ContentStatus, ServiceType};
use serde_json::{Value, json};

use crate::controller::Controller;
use crate::db::{mappings, master_tags, notes, ratings};
use crate::metadata::Rating;
use crate::networking::client_networking_url_class::UrlClass;

use super::{AppState, UiCommand, query, render};

const RATING_TYPES: &[ServiceType] = &[
    ServiceType::LocalRatingNumerical,
    ServiceType::LocalRatingLike,
    ServiceType::LocalRatingIncDec,
];

pub struct TagServiceView {
    pub key: String,
    pub name: String,
    pub tags: Vec<String>,
    /// Tags after sibling resolution. Two storage tags that are siblings
    /// collapse onto one display tag, which is what Hydrus shows in most
    /// panels by default.
    pub display_tags: Vec<String>,
    /// Display tag -> storage tags that resolve to it (for tooltips).
    pub display_tag_sources: BTreeMap<String, Vec<String>>,
}

pub struct RatingServiceView {
    pub key: String,
    pub name: String,
    pub kind: String,
    pub value: Value,
}

pub struct NoteView {
    pub name: String,
    pub text: String,
}

pub fn metadata_json(controller: &Controller, hash: &[u8]) -> Value {
    let Some(hash_id) = query::hash_id_for(controller, hash) else {
        return json!({"hash": query::hex(hash), "tag_services": [], "ratings": [], "notes": []});
    };

    let tag_services: Vec<Value> = tag_services(controller, hash_id)
        .into_iter()
        .map(|service| {
            json!({
                "key": service.key,
                "name": service.name,
                "tags": service.tags,
                "display_tags": service.display_tags,
                "display_tag_sources": service.display_tag_sources,
            })
        })
        .collect();

    let ratings: Vec<Value> = rating_services(controller, hash_id)
        .into_iter()
        .map(|service| {
            json!({
                "key": service.key,
                "name": service.name,
                "kind": service.kind,
                "value": service.value,
            })
        })
        .collect();

    let notes: Vec<Value> = notes::get_notes(controller.db().db(), hash_id)
        .unwrap_or_default()
        .into_iter()
        .map(|(name, text)| json!({"name": name, "text": text}))
        .collect();

    json!({
        "hash": query::hex(hash),
        "tag_services": tag_services,
        "ratings": ratings,
        "notes": notes,
    })
}

pub fn tag_services(controller: &Controller, hash_id: i64) -> Vec<TagServiceView> {
    let manager = controller.services();
    let db = controller.db().db();
    let mut services: Vec<TagServiceView> = manager
        .get_services(Some(&[ServiceType::LocalTag]))
        .into_iter()
        .map(|service| {
            let mut tags =
                mappings::get_tags(db, service.service_id, ContentStatus::Current, hash_id)
                    .unwrap_or_default();
            tags.sort();
            let display_tags = crate::db::tag_display::get_display_tags(
                db,
                service.service_id,
                tags.iter().cloned(),
            )
            .unwrap_or_else(|_| tags.clone());
            let mut display_tag_sources = BTreeMap::new();

            for tag in &tags {
                let display = crate::db::tag_display::get_display_tag(db, service.service_id, tag)
                    .unwrap_or_else(|_| tag.clone());
                display_tag_sources
                    .entry(display)
                    .or_insert_with(Vec::new)
                    .push(tag.clone());
            }

            TagServiceView {
                key: query::hex(&service.service_key),
                name: service.name.clone(),
                tags,
                display_tags,
                display_tag_sources,
            }
        })
        .collect();

    services.sort_by(|a, b| {
        let local = |name: &str| name.eq_ignore_ascii_case("local tags");
        local(&b.name)
            .cmp(&local(&a.name))
            .then(a.name.cmp(&b.name))
    });

    services
}

pub fn rating_services(controller: &Controller, hash_id: i64) -> Vec<RatingServiceView> {
    let manager = controller.services();
    let db = controller.db().db();

    let mut services: Vec<RatingServiceView> = manager
        .get_services(Some(RATING_TYPES))
        .into_iter()
        .map(|service| {
            let rating = ratings::get_rating(db, service.service_id, hash_id).unwrap_or(None);

            let (kind, value) = match service.service_type {
                ServiceType::LocalRatingNumerical => (
                    "numerical",
                    json!(match rating {
                        Some(Rating::Numerical(value)) => value,
                        _ => 0.0,
                    }),
                ),
                ServiceType::LocalRatingLike => (
                    "like",
                    match rating {
                        Some(Rating::Like) => json!("like"),
                        Some(Rating::Dislike) => json!("dislike"),
                        _ => Value::Null,
                    },
                ),
                _ => (
                    "incdec",
                    json!(match rating {
                        Some(Rating::Numerical(value)) => value as i64,
                        Some(Rating::Like) => 1,
                        Some(Rating::Dislike) => -1,
                        None => 0,
                    }),
                ),
            };

            RatingServiceView {
                key: query::hex(&service.service_key),
                name: service.name.clone(),
                kind: kind.to_string(),
                value,
            }
        })
        .collect();

    services.sort_by(|a, b| a.name.cmp(&b.name));

    services
}

pub fn notes_json(controller: &Controller, hash: &[u8]) -> Value {
    metadata_json(controller, hash)
}

pub fn tag_service_id(controller: &Controller, key_or_name: &str) -> Option<i64> {
    let manager = controller.services();

    query::unhex(key_or_name)
        .and_then(|key| manager.get_service(&key).map(|service| service.service_id))
        .or_else(|| {
            manager
                .get_service_key_from_name(key_or_name)
                .and_then(|key| manager.get_service(key).map(|service| service.service_id))
        })
        .or_else(|| query::tag_service_id(controller))
}

pub fn rating_service(controller: &Controller, key: &str) -> Option<(i64, ServiceType)> {
    let key = query::unhex(key)?;
    let manager = controller.services();
    let service = manager.get_service(&key)?;

    if !RATING_TYPES.contains(&service.service_type) {
        return None;
    }

    Some((service.service_id, service.service_type))
}

pub fn set_tag(
    state: &mut AppState,
    hash: &[u8],
    service: &str,
    tag: &str,
    add: bool,
) -> Vec<UiCommand> {
    let Some(hash_id) = query::hash_id_for(&state.controller, hash) else {
        return vec![UiCommand::NotifyMessage(
            "file is not in the database".to_string(),
        )];
    };

    let tag = tag.trim();

    if tag.is_empty() {
        return vec![];
    }

    let Some(service_id) = tag_service_id(&state.controller, service) else {
        return vec![UiCommand::NotifyMessage("no local tag service".to_string())];
    };

    let db = state.controller.db().db();
    let Ok(tag_id) = master_tags::get_tag_id(db, tag) else {
        return vec![UiCommand::NotifyMessage(format!(
            "could not read tag '{tag}'"
        ))];
    };

    let result = if add {
        mappings::add_mappings(db, service_id, ContentStatus::Current, hash_id, &[tag_id])
    } else {
        mappings::remove_mappings(db, service_id, ContentStatus::Current, hash_id, &[tag_id])
    };

    if let Err(error) = result {
        return vec![UiCommand::NotifyMessage(format!(
            "could not update tags: {error}"
        ))];
    }

    state.message = if add {
        format!("added tag '{tag}'")
    } else {
        format!("removed tag '{tag}'")
    };

    refresh_hash(state, hash)
}

pub fn set_rating(
    state: &mut AppState,
    hash: &[u8],
    service_key: &str,
    action: &str,
    value: f64,
) -> Vec<UiCommand> {
    let Some(hash_id) = query::hash_id_for(&state.controller, hash) else {
        return vec![UiCommand::NotifyMessage(
            "file is not in the database".to_string(),
        )];
    };

    let Some((service_id, service_type)) = rating_service(&state.controller, service_key) else {
        return vec![UiCommand::NotifyMessage(
            "rating service is missing".to_string(),
        )];
    };

    let db = state.controller.db().db();
    let result = match action {
        "clear" => ratings::delete_rating(db, service_id, hash_id),
        "like" => ratings::set_rating(db, service_id, hash_id, Rating::Like),
        "dislike" => ratings::set_rating(db, service_id, hash_id, Rating::Dislike),
        _ if service_type == ServiceType::LocalRatingIncDec => {
            ratings::set_incdec_rating(db, service_id, hash_id, value.round() as i64)
        }
        _ => ratings::set_rating(
            db,
            service_id,
            hash_id,
            Rating::Numerical(value.clamp(0.0, 1.0)),
        ),
    };

    if let Err(error) = result {
        return vec![UiCommand::NotifyMessage(format!(
            "could not save the rating: {error}"
        ))];
    }

    state.message = "rating updated".to_string();
    refresh_hash(state, hash)
}

pub fn manage_notes(state: &mut AppState, hash: &[u8]) -> Vec<UiCommand> {
    let Some(hash_id) = query::hash_id_for(&state.controller, hash) else {
        return vec![UiCommand::NotifyMessage(
            "file is not in the database".to_string(),
        )];
    };

    let notes = notes::get_notes(state.controller.db().db(), hash_id).unwrap_or_default();
    let notes: Vec<Value> = notes
        .into_iter()
        .map(|(name, text)| json!({"name": name, "text": text}))
        .collect();
    let dialog = json!({
        "dialog_id": "notes",
        "hash": query::hex(hash),
        "notes": notes,
    });

    vec![UiCommand::NotesDialog(dialog.to_string())]
}

pub fn save_notes(
    state: &mut AppState,
    hash: &[u8],
    values: &BTreeMap<String, String>,
) -> Vec<UiCommand> {
    let Some(hash_id) = query::hash_id_for(&state.controller, hash) else {
        return vec![UiCommand::NotifyMessage(
            "file is not in the database".to_string(),
        )];
    };

    let db = state.controller.db().db();
    let before = notes::get_notes(db, hash_id).unwrap_or_default();

    for name in before.keys() {
        if !values.contains_key(name) {
            let _ = notes::delete_note(db, hash_id, name);
        }
    }

    for (name, text) in values {
        let name = name.trim();

        if name.is_empty() {
            continue;
        }

        if text.trim().is_empty() {
            let _ = notes::delete_note(db, hash_id, name);
        } else if let Err(error) = notes::set_note(db, hash_id, name, text) {
            return vec![UiCommand::NotifyMessage(format!(
                "could not save note '{name}': {error}"
            ))];
        }
    }

    state.message = "notes saved".to_string();
    refresh_hash(state, hash)
}

/// Re-renders every visible page and viewer that shows `hash`.
pub fn refresh_hash(state: &mut AppState, hash: &[u8]) -> Vec<UiCommand> {
    let mut commands = state.refresh_commands();

    for (viewer_id, context) in &state.viewers {
        if context.hash == hash {
            if let Some(page) = state.find_query(&context.page_id) {
                commands.push(UiCommand::UpdateViewer(
                    viewer_id.clone(),
                    render::viewer_json(&state.controller, viewer_id, page, hash),
                ));
            }
        }
    }

    if let Some(page) = state.active_query() {
        if page.focused.as_deref() == Some(hash) {
            let page_id = page.id.clone();
            commands.extend(state.focus(&page_id, &query::hex(hash)));
        }
    }

    commands
}

pub fn tag_action(state: &mut AppState, add: bool, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return vec![UiCommand::NotifyMessage("invalid tag payload".to_string())];
    };

    let Some(hash) = value
        .get("hash")
        .and_then(Value::as_str)
        .and_then(query::unhex)
    else {
        return vec![UiCommand::NotifyMessage("invalid file hash".to_string())];
    };

    let service = value
        .get("service")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let tag = value.get("tag").and_then(Value::as_str).unwrap_or_default();

    set_tag(state, &hash, service, tag, add)
}

pub fn rating_action(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return vec![UiCommand::NotifyMessage(
            "invalid rating payload".to_string(),
        )];
    };

    let Some(hash) = value
        .get("hash")
        .and_then(Value::as_str)
        .and_then(query::unhex)
    else {
        return vec![UiCommand::NotifyMessage("invalid file hash".to_string())];
    };

    let service = value
        .get("service")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let action = value
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("clear");
    let rating = value.get("value").and_then(Value::as_f64).unwrap_or(0.0);

    set_rating(state, &hash, service, action, rating)
}

pub fn notes_action(state: &mut AppState, payload: &str, save: bool) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return vec![UiCommand::NotifyMessage(
            "invalid notes payload".to_string(),
        )];
    };

    let Some(hash) = value
        .get("hash")
        .and_then(Value::as_str)
        .and_then(query::unhex)
    else {
        return vec![UiCommand::NotifyMessage("invalid file hash".to_string())];
    };

    if !save {
        return manage_notes(state, &hash);
    }

    let mut values = BTreeMap::new();

    if let Some(notes) = value.get("notes") {
        // Accept either a list of `{name,text}` objects from the editor or a
        // plain object for scripted use.
        if let Some(array) = notes.as_array() {
            for note in array {
                let name = note.get("name").and_then(Value::as_str).unwrap_or_default();
                let text = note.get("text").and_then(Value::as_str).unwrap_or_default();
                values.insert(name.to_string(), text.to_string());
            }
        } else if let Some(object) = notes.as_object() {
            for (name, text) in object {
                values.insert(name.clone(), text.as_str().unwrap_or_default().to_string());
            }
        }
    }

    save_notes(state, &hash, &values)
}

/// Resolves the active selection, for menu actions.
pub fn active_hash(state: &AppState) -> Option<(String, Vec<u8>)> {
    let page = state.active_query()?;
    let hash = page.focused.clone()?;

    Some((page.id.clone(), hash))
}

pub fn manage_notes_active(state: &mut AppState) -> Vec<UiCommand> {
    match active_hash(state) {
        Some((_, hash)) => manage_notes(state, &hash),
        None => vec![UiCommand::NotifyMessage("no file selected".to_string())],
    }
}

pub fn url_class_from_json(value: &Value, existing: Option<&UrlClass>) -> Option<UrlClass> {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())?;

    let netloc = value
        .get("netloc")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .trim_start_matches("www.")
        .to_ascii_lowercase();

    if netloc.is_empty() {
        return None;
    }

    let mut class = existing
        .cloned()
        .unwrap_or_else(|| UrlClass::new(name, netloc.clone()));
    class.name = name.to_string();
    class.netloc = netloc;
    class.url_type = value
        .get("url_type")
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .unwrap_or(class.url_type);
    class.preferred_scheme = value
        .get("preferred_scheme")
        .and_then(Value::as_str)
        .filter(|scheme| !scheme.trim().is_empty())
        .map(|scheme| scheme.trim().to_ascii_lowercase())
        .or(Some("https".to_string()));
    class.match_subdomains = value
        .get("match_subdomains")
        .and_then(Value::as_bool)
        .unwrap_or(class.match_subdomains);
    class.keep_matched_subdomains = value
        .get("keep_matched_subdomains")
        .and_then(Value::as_bool)
        .unwrap_or(class.keep_matched_subdomains);
    class.can_produce_multiple_files = value
        .get("can_produce_multiple_files")
        .and_then(Value::as_bool)
        .unwrap_or(class.can_produce_multiple_files);
    class.should_be_associated_with_files = value
        .get("should_be_associated_with_files")
        .and_then(Value::as_bool)
        .unwrap_or(class.should_be_associated_with_files);
    class.example_url = value
        .get("example_url")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    if let Some(path) = value.get("path").and_then(Value::as_str) {
        class.path_components = path
            .split('/')
            .map(str::trim)
            .filter(|segment| !segment.is_empty())
            .map(|segment| {
                if segment.starts_with('{') && segment.ends_with('}') {
                    crate::networking::client_networking_url_class::PathComponent::Variable {
                        name: segment.trim_matches(['{', '}']).to_string(),
                        default: String::new(),
                    }
                } else {
                    crate::networking::client_networking_url_class::PathComponent::Fixed(
                        segment.to_string(),
                    )
                }
            })
            .collect();
    }

    if let Some(parameters) = value.get("parameters").and_then(Value::as_str) {
        class.parameters = parameters
            .lines()
            .filter_map(|line| {
                let line = line.trim();

                if line.is_empty() {
                    None
                } else if let Some((name, value)) = line.split_once('=') {
                    Some((name.trim().to_string(), Some(value.trim().to_string())))
                } else {
                    Some((line.to_string(), None))
                }
            })
            .collect();
    }

    Some(class)
}

#[cfg(test)]
mod tests;
