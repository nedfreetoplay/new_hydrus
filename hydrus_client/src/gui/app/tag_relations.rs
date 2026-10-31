//! Tag siblings and parents editor.
//!
//! The client database already stores and resolves relations
//! (`db::tag_relations`); this module is the editing surface behind the
//! "manage tag siblings/parents" dialogs and keeps the display-tag panels in
//! sync. Siblings collapse a tag onto its preferred spelling, parents describe
//! a `child -> parent` hierarchy.

use hydrus_core::hydrus_constants::{ContentStatus, ServiceType};
use serde_json::{Value, json};

use crate::db::{
    master_tags,
    tag_relations::{self, TagRelation},
};

use super::{AppState, UiCommand, metadata, query, render};

pub const SIBLINGS: &str = "siblings";
pub const PARENTS: &str = "parents";

fn relation_from_name(name: &str) -> Option<TagRelation> {
    match name.trim().to_ascii_lowercase().as_str() {
        SIBLINGS | "sibling" | "tag_siblings" => Some(TagRelation::Siblings),
        PARENTS | "parent" | "tag_parents" => Some(TagRelation::Parents),
        _ => None,
    }
}

fn relation_name(relation: TagRelation) -> &'static str {
    match relation {
        TagRelation::Siblings => SIBLINGS,
        TagRelation::Parents => PARENTS,
    }
}

fn relation_json(
    relation: TagRelation,
    service_key: &str,
    pairs: Vec<(String, String)>,
) -> Vec<Value> {
    pairs
        .into_iter()
        .map(|(first, second)| {
            json!({
                "relation": relation_name(relation),
                "service": service_key,
                "first": first,
                "second": second,
            })
        })
        .collect()
}

/// Everything the relations dialog needs. Relations for every local tag
/// service are sent at once so switching service in the dialog is instant.
pub fn dialog_json(state: &AppState, tab: &str) -> String {
    let manager = state.controller.services();
    let db = state.controller.db().db();
    let mut services = Vec::new();
    let mut relations = Vec::new();

    for service in manager.get_services(Some(&[ServiceType::LocalTag])) {
        let key = query::hex(&service.service_key);
        services.push(json!({"key": key, "name": service.name}));

        for relation in [TagRelation::Siblings, TagRelation::Parents] {
            let pairs = tag_relations::get_pairs_with_tags(
                db,
                relation,
                service.service_id,
                ContentStatus::Current,
            )
            .unwrap_or_default();

            relations.extend(relation_json(relation, &key, pairs));
        }
    }

    json!({
        "dialog_id": "tag_relations",
        "tab": if tab == PARENTS { PARENTS } else { SIBLINGS },
        "services": services,
        "relations": relations,
    })
    .to_string()
}

pub fn show(state: &AppState, tab: &str) -> Vec<UiCommand> {
    vec![UiCommand::TagRelationsDialog(dialog_json(state, tab))]
}

pub fn action(state: &mut AppState, payload: &str, add: bool) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return vec![UiCommand::NotifyMessage(
            "invalid tag relation payload".to_string(),
        )];
    };

    let Some(relation) = value
        .get("relation")
        .and_then(Value::as_str)
        .and_then(relation_from_name)
    else {
        return vec![UiCommand::NotifyMessage(
            "tag relation type is missing".to_string(),
        )];
    };

    let first = value
        .get("first")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let second = value
        .get("second")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let service = value
        .get("service")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if first.is_empty() || second.is_empty() {
        return vec![UiCommand::NotifyMessage(
            "both tags are required".to_string(),
        )];
    }

    if first.eq_ignore_ascii_case(second) {
        return vec![UiCommand::NotifyMessage(
            "a tag cannot be related to itself".to_string(),
        )];
    }

    let Some(service_id) = metadata::tag_service_id(&state.controller, service) else {
        return vec![UiCommand::NotifyMessage("no local tag service".to_string())];
    };

    let db = state.controller.db().db();
    let result = if add {
        add_pair(db, relation, service_id, first, second)
    } else {
        remove_pair(db, relation, service_id, first, second)
    };

    match result {
        Ok(message) => state.message = message,
        Err(error) => {
            return vec![UiCommand::NotifyMessage(format!(
                "could not update tag relations: {error}"
            ))];
        }
    }

    let mut commands = state.refresh_commands();

    for (viewer_id, context) in &state.viewers {
        if let Some(page) = state.find_query(&context.page_id) {
            commands.push(UiCommand::UpdateViewer(
                viewer_id.clone(),
                render::viewer_json(&state.controller, viewer_id, page, &context.hash),
            ));
        }
    }

    commands.push(UiCommand::TagRelationsDialog(dialog_json(
        state,
        relation_name(relation),
    )));
    commands.push(UiCommand::Status(render::status(state)));
    commands
}

fn add_pair(
    db: &hydrus_core::hydrus_db_base::DBBase,
    relation: TagRelation,
    service_id: i64,
    first: &str,
    second: &str,
) -> Result<String, String> {
    // Reject direct cycles before writing: a sibling chain that falls back onto
    // itself or a parent that is already a descendant would loop the resolver.
    match relation {
        TagRelation::Siblings => {
            let resolved = tag_relations::resolve_siblings(db, service_id, second)
                .map_err(|error| error.to_string())?;

            if resolved == first {
                return Err(format!(
                    "'{first}' and '{second}' already resolve to each other"
                ));
            }
        }
        TagRelation::Parents => {
            let ancestors = tag_relations::get_ancestors(db, service_id, second)
                .map_err(|error| error.to_string())?;

            if ancestors.contains(first) {
                return Err(format!("'{first}' is already a parent of '{second}'"));
            }
        }
    }

    let first_id = master_tags::get_tag_id(db, first).map_err(|error| error.to_string())?;
    let second_id = master_tags::get_tag_id(db, second).map_err(|error| error.to_string())?;

    tag_relations::add_pair(
        db,
        relation,
        service_id,
        ContentStatus::Current,
        first_id,
        second_id,
        None,
    )
    .map_err(|error| error.to_string())?;

    Ok(match relation {
        TagRelation::Siblings => format!("'{first}' -> '{second}' sibling saved"),
        TagRelation::Parents => format!("'{first}' -> '{second}' parent saved"),
    })
}

fn remove_pair(
    db: &hydrus_core::hydrus_db_base::DBBase,
    relation: TagRelation,
    service_id: i64,
    first: &str,
    second: &str,
) -> Result<String, String> {
    let known = master_tags::has_tag(db, first).map_err(|error| error.to_string())?
        && master_tags::has_tag(db, second).map_err(|error| error.to_string())?;

    if !known {
        return Ok("tag relation was not found".to_string());
    }

    let first_id = master_tags::get_tag_id(db, first).map_err(|error| error.to_string())?;
    let second_id = master_tags::get_tag_id(db, second).map_err(|error| error.to_string())?;

    tag_relations::remove_pair(
        db,
        relation,
        service_id,
        ContentStatus::Current,
        first_id,
        second_id,
    )
    .map_err(|error| error.to_string())?;

    Ok(match relation {
        TagRelation::Siblings => format!("sibling '{first}' -> '{second}' removed"),
        TagRelation::Parents => format!("parent '{first}' -> '{second}' removed"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_names_round_trip() {
        assert_eq!(relation_from_name("siblings"), Some(TagRelation::Siblings));
        assert_eq!(relation_from_name("parents"), Some(TagRelation::Parents));

        for relation in [TagRelation::Siblings, TagRelation::Parents] {
            assert_eq!(relation_from_name(relation_name(relation)), Some(relation));
        }

        assert_eq!(relation_from_name("uncles"), None);
    }

    #[test]
    fn pairs_are_rendered_with_their_service_and_kind() {
        let rows = relation_json(
            TagRelation::Parents,
            "aabb",
            vec![("child".to_string(), "parent".to_string())],
        );

        assert_eq!(rows[0]["relation"], PARENTS);
        assert_eq!(rows[0]["service"], "aabb");
        assert_eq!(rows[0]["first"], "child");
        assert_eq!(rows[0]["second"], "parent");
    }
}
