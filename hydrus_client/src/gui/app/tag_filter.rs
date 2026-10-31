//! Tag filter editing and tag purging for tag services.
//!
//! A tag repository serves its filter through `/tag_filter`; the client also
//! keeps a copy in the service dictionary so the dialog can show rules while
//! offline. Local tag services only keep the dictionary copy. Purging removes
//! mappings from a local tag service and reports how many mappings and tags
//! were affected.

use hydrus_core::hydrus_constants::{ContentStatus, Filter, ServiceType};
use hydrus_core::hydrus_tags::{self, TagFilter};
use serde_json::{json, Value};

use crate::db::services::Service;
use crate::networking::hydrus_admin::HydrusServiceClient;

use super::{render, services, AppState, UiCommand};

/// The wildcard tokens the dialog uses for the two "all tags" slices, matching
/// the Python tag filter UI.
const ALL_UNNAMESPACED_TOKEN: &str = "*";
const ALL_NAMESPACED_TOKEN: &str = "*:*";

enum Target {
    Local(Service),
    Remote {
        key: String,
        name: String,
        port: u16,
    },
}

impl Target {
    fn name(&self) -> &str {
        match self {
            Self::Local(service) => &service.name,
            Self::Remote { name, .. } => name,
        }
    }

    fn is_remote(&self) -> bool {
        match self {
            Self::Local(service) => service.service_type == ServiceType::TagRepository,
            Self::Remote { .. } => true,
        }
    }

    fn is_tag_service(&self) -> bool {
        match self {
            Self::Local(service) => is_tag_service(service),
            Self::Remote { .. } => true,
        }
    }

    fn can_purge(&self) -> bool {
        matches!(
            self,
            Self::Local(service) if service.service_type == ServiceType::LocalTag
        )
    }
}

pub fn open(state: &AppState, payload: &str) -> Vec<UiCommand> {
    let key = payload.trim();
    let Some(target) = resolve(state, key) else {
        return notify(state, "service is missing");
    };

    if !target.is_tag_service() {
        return notify(state, format!("'{}' is not a tag service", target.name()));
    }

    let mut rules = match &target {
        Target::Local(service) => rules_from_dictionary(&services::service_dictionary(service)),
        Target::Remote { .. } => Vec::new(),
    };
    let mut note = String::new();

    if target.is_remote() {
        match remote_client(state, &target) {
            Ok(client) => match client.tag_filter() {
                Ok(server_rules) => rules = server_rules,
                Err(error) => note = format!("could not read the server filter: {error}"),
            },
            Err(error) => note = format!("could not connect to the repository: {error}"),
        }
    }

    let filter = filter_from_rules(&rules);

    let entries: Vec<Value> = rules
        .iter()
        .map(|(tag_slice, rule)| {
            json!({
                "slice": tag_slice,
                "display": display_slice(tag_slice),
                "rule": rule,
            })
        })
        .collect();

    let value = json!({
        "service_key": match &target {
            Target::Local(service) => services::hex_encode(&service.service_key),
            Target::Remote { key, .. } => key.clone(),
        },
        "name": target.name(),
        "remote": target.is_remote(),
        "can_purge": target.can_purge(),
        "rules": entries,
        "filter_string": filter.to_filter_string(),
        "permitted_string": filter.to_permitted_string(),
        "note": note,
    });

    vec![UiCommand::TagFilterDialog(value.to_string())]
}

pub fn save(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return notify(state, "invalid tag filter payload");
    };
    let key = value
        .get("service_key")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    let Some(target) = resolve(state, &key) else {
        return notify(state, "service is missing");
    };

    if !target.is_tag_service() {
        return notify(state, format!("'{}' is not a tag service", target.name()));
    }

    let rules = match parse_rules(&value) {
        Ok(rules) => rules,
        Err(error) => return notify(state, error),
    };

    if target.is_remote() {
        let client = match remote_client(state, &target) {
            Ok(client) => client,
            Err(error) => return notify(state, format!("repository error: {error}")),
        };

        if let Err(error) = client.set_tag_filter(&rules) {
            return notify(state, format!("repository refused the tag filter: {error}"));
        }
    }

    if let Target::Local(service) = &target {
        let mut dictionary = services::service_dictionary(service);
        let serialised: Vec<Value> = rules
            .iter()
            .map(|(slice, rule)| json!([slice, rule]))
            .collect();
        dictionary["tag_filter"] = Value::Array(serialised);

        if let Err(error) = state.controller.update_service(
            &services::hex_encode(&service.service_key),
            &service.name,
            &dictionary.to_string(),
        ) {
            return notify(state, format!("could not save the tag filter: {error}"));
        }
    }

    refresh_menus(state)
        .into_iter()
        .chain([UiCommand::NotifyMessage(format!(
            "tag filter for '{}' saved",
            target.name()
        ))])
        .collect()
}

pub fn test(state: &AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return notify(state, "invalid tag filter payload");
    };
    let tag = value
        .get("tag")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let rules = match parse_rules(&value) {
        Ok(rules) => rules,
        Err(error) => return notify(state, error),
    };

    if tag.is_empty() {
        return notify(state, "enter a tag to test");
    }

    let Ok(tag) = hydrus_tags::clean_tag(tag) else {
        return notify(state, format!("'{tag}' is not a valid tag"));
    };

    let filter = filter_from_rules(&rules);
    let passes = filter.tag_ok(&tag, false);
    let verdict = if passes { "passes" } else { "does not pass" };

    notify(state, format!("'{tag}' {verdict} the filter"))
}

pub fn purge(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return notify(state, "invalid purge payload");
    };
    let key = value
        .get("service_key")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    let Some(Target::Local(service)) = resolve(state, &key) else {
        return notify(
            state,
            "purging is only available for local tag services in this build",
        );
    };

    if service.service_type != ServiceType::LocalTag {
        return notify(
            state,
            "purging is only available for local tag services in this build",
        );
    }

    let Some(tags) = value.get("tags").and_then(Value::as_array) else {
        return notify(state, "enter tags to purge");
    };

    let cleaned =
        match hydrus_tags::clean_tags(tags.iter().filter_map(Value::as_str).map(str::to_string)) {
            Ok(tags) => tags,
            Err(error) => return notify(state, format!("could not parse the tags: {error}")),
        };

    if cleaned.is_empty() {
        return notify(state, "enter tags to purge");
    }

    let db = state.controller.db().db();
    let mut tag_ids = Vec::new();

    for tag in &cleaned {
        match crate::db::master_tags::get_tag_id_if_exists(db, tag) {
            Ok(Some(tag_id)) => tag_ids.push(tag_id),
            Ok(None) => {}
            Err(error) => return notify(state, format!("database error: {error}")),
        }
    }

    if tag_ids.is_empty() {
        return notify(state, "none of those tags exist on the service");
    }

    let mut removed = 0usize;

    for status in [
        ContentStatus::Current,
        ContentStatus::Pending,
        ContentStatus::Deleted,
        ContentStatus::Petitioned,
    ] {
        for tag_id in &tag_ids {
            match crate::db::mappings::remove_tag_from_service(
                db,
                service.service_id,
                status,
                *tag_id,
            ) {
                Ok(count) => removed += count,
                Err(error) => return notify(state, format!("database error: {error}")),
            }
        }
    }

    let reason = value
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    let mut message = format!(
        "purged {} tag(s) from '{}', removing {} mapping(s)",
        tag_ids.len(),
        service.name,
        removed
    );

    if !reason.is_empty() {
        message.push_str(&format!(" ({reason})"));
    }

    clean_caches(state);
    notify(state, message)
}

fn resolve(state: &AppState, key: &str) -> Option<Target> {
    let wanted = key.trim().to_ascii_lowercase();

    if let Some(service) = state
        .controller
        .services()
        .get_services(None)
        .into_iter()
        .find(|service| wanted.is_empty() || services::hex_encode(&service.service_key) == wanted)
    {
        return Some(Target::Local(service.clone()));
    }

    state
        .admin
        .services
        .iter()
        .find(|service| service.key.to_ascii_lowercase() == wanted)
        .map(|service| Target::Remote {
            key: service.key.clone(),
            name: service.name.clone(),
            port: service.port,
        })
}

fn is_tag_service(service: &Service) -> bool {
    matches!(
        service.service_type,
        ServiceType::TagRepository | ServiceType::LocalTag
    )
}

fn remote_client(state: &AppState, target: &Target) -> Result<HydrusServiceClient, String> {
    let (key, port, dictionary) = match target {
        Target::Local(service) => (
            services::hex_encode(&service.service_key),
            services::service_dictionary(service)
                .get("port")
                .and_then(Value::as_u64)
                .and_then(|port| u16::try_from(port).ok())
                .unwrap_or(0),
            services::service_dictionary(service),
        ),
        Target::Remote { key, port, .. } => (key.clone(), *port, Value::Null),
    };

    let access_key = dictionary
        .get("access_key")
        .and_then(Value::as_str)
        .filter(|key| !key.trim().is_empty())
        .map(str::to_string)
        .or_else(|| state.admin.service_keys.get(&key).cloned())
        .or_else(|| {
            if state.admin.access_key.trim().is_empty() {
                None
            } else {
                Some(state.admin.access_key.clone())
            }
        })
        .ok_or_else(|| "the service has no access key".to_string())?;

    let address = if !state.admin.address.trim().is_empty() {
        state.admin.address.clone()
    } else {
        let host = dictionary
            .get("host")
            .and_then(Value::as_str)
            .unwrap_or("127.0.0.1");

        format!("https://{host}")
    };

    let client = HydrusServiceClient::new(&address, &access_key)?;

    Ok(if port == 0 {
        client
    } else {
        client.with_port(port)
    })
}

fn rules_from_dictionary(dictionary: &Value) -> Vec<(String, String)> {
    let mut rules = Vec::new();

    if let Some(entries) = dictionary.get("tag_filter").and_then(Value::as_array) {
        for entry in entries {
            let Some(pair) = entry.as_array() else {
                continue;
            };
            let (Some(slice), Some(rule)) = (pair.first(), pair.get(1)) else {
                continue;
            };

            let (Some(slice), Some(rule)) = (slice.as_str(), rule.as_str()) else {
                continue;
            };

            if rule == "whitelist" || rule == "blacklist" {
                rules.push((slice.to_string(), rule.to_string()));
            }
        }
    }

    rules
}

pub(super) fn parse_rules(value: &Value) -> Result<Vec<(String, String)>, String> {
    let Some(entries) = value.get("rules").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };

    let mut rules = Vec::with_capacity(entries.len());

    for entry in entries {
        let slice = entry
            .get("slice")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        let rule = entry
            .get("rule")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();

        if rule != "whitelist" && rule != "blacklist" {
            return Err(format!("'{rule}' is not a tag filter rule"));
        }

        rules.push((slice.to_string(), rule.to_string()));
    }

    Ok(rules)
}

pub(super) fn filter_from_rules(rules: &[(String, String)]) -> TagFilter {
    let mut filter = TagFilter::new();

    for (slice, rule) in rules {
        let rule = if rule == "whitelist" {
            Filter::Whitelist
        } else {
            Filter::Blacklist
        };
        filter.set_rule(slice, rule);
    }

    filter.clean_rules();
    filter
}

pub(super) fn display_slice(tag_slice: &str) -> String {
    match tag_slice {
        "" => ALL_UNNAMESPACED_TOKEN.to_string(),
        ":" => ALL_NAMESPACED_TOKEN.to_string(),
        _ => tag_slice.to_string(),
    }
}

fn notify(state: &AppState, message: impl Into<String>) -> Vec<UiCommand> {
    let mut commands = vec![UiCommand::NotifyMessage(message.into())];
    commands.extend(refresh_menus(state));
    commands
}

fn refresh_menus(state: &AppState) -> Vec<UiCommand> {
    vec![
        UiCommand::Menus(render::menus_json(state)),
        UiCommand::Status(render::status(state)),
    ]
}

fn clean_caches(state: &AppState) {
    let _ = state.controller.reload_services();
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::gui::app::AppState;

    fn test_state(directory: &std::path::Path) -> AppState {
        let _ = crate::client_options::register_serialisable_type();
        let controller = crate::controller::Controller::new(directory).unwrap();
        controller.init_model().unwrap();

        AppState::new(Arc::new(controller))
    }

    fn temp_directory(label: &str) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_tag_filter_{label}_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    fn local_tag_key(state: &AppState) -> String {
        let manager = state.controller.services();
        let service = manager
            .get_services(None)
            .into_iter()
            .find(|service| service.service_type == ServiceType::LocalTag)
            .unwrap();

        services::hex_encode(&service.service_key)
    }

    #[test]
    fn filter_rules_round_trip_through_json() {
        let rules = vec![
            (String::new(), "blacklist".to_string()),
            (":".to_string(), "blacklist".to_string()),
            ("character:".to_string(), "whitelist".to_string()),
        ];
        let filter = filter_from_rules(&rules);

        assert!(filter.tag_ok("character:samus", false));
        assert!(!filter.tag_ok("species:human", false));
        assert!(!filter.tag_ok("samus", false));

        let value = json!({
            "rules": rules
                .iter()
                .map(|(slice, rule)| json!({"slice": slice, "rule": rule}))
                .collect::<Vec<_>>(),
        });
        let parsed = parse_rules(&value).unwrap();

        assert_eq!(parsed, rules);
    }

    #[test]
    fn wildcards_display_as_all_tags() {
        assert_eq!(display_slice(""), "*");
        assert_eq!(display_slice(":"), "*:*");
        assert_eq!(display_slice("creator"), "creator");
    }

    #[test]
    fn saving_a_filter_persists_it_in_the_service_dictionary() {
        let directory = temp_directory("save");
        let mut state = test_state(&directory);
        let key = local_tag_key(&state);

        let payload = json!({
            "service_key": key,
            "rules": [
                {"slice": "", "rule": "blacklist"},
                {"slice": ":", "rule": "blacklist"},
                {"slice": "character:", "rule": "whitelist"},
            ],
        })
        .to_string();

        let commands = save(&mut state, &payload);
        assert!(
            commands.iter().any(|command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("saved"))),
            "{commands:?}"
        );

        let manager = state.controller.services();
        let service = manager
            .get_services(None)
            .into_iter()
            .find(|service| service.service_type == ServiceType::LocalTag)
            .unwrap();
        let rules = rules_from_dictionary(&services::service_dictionary(service));

        assert_eq!(rules.len(), 3);
        assert!(rules
            .iter()
            .any(|(slice, rule)| slice == "character:" && rule == "whitelist"));

        let opened = open(&state, &key);
        let json = match opened.first() {
            Some(UiCommand::TagFilterDialog(json)) => json,
            other => panic!("unexpected command: {other:?}"),
        };
        let value: Value = serde_json::from_str(json).unwrap();

        assert_eq!(value["rules"].as_array().unwrap().len(), 3);
        assert_eq!(value["note"], "");

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn invalid_rules_are_rejected() {
        let value = json!({"rules": [{"slice": "x", "rule": "maybe"}]});

        assert!(parse_rules(&value).is_err());
    }

    #[test]
    fn purging_tags_removes_local_mappings() {
        let directory = temp_directory("purge");
        let mut state = test_state(&directory);

        let hash_id = 7;
        let tag = "character:samus";
        let tag_id = crate::db::master_tags::get_tag_id(state.controller.db().db(), tag).unwrap();
        let manager = state.controller.services();
        let service = manager
            .get_services(None)
            .into_iter()
            .find(|service| service.service_type == ServiceType::LocalTag)
            .unwrap()
            .clone();

        crate::db::mappings::add_mappings(
            state.controller.db().db(),
            service.service_id,
            ContentStatus::Current,
            hash_id,
            &[tag_id],
        )
        .unwrap();

        let payload = json!({
            "service_key": services::hex_encode(&service.service_key),
            "tags": [tag],
            "reason": "test purge",
        })
        .to_string();

        let commands = purge(&mut state, &payload);
        assert!(
            commands.iter().any(|command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("removing 1 mapping"))),
            "{commands:?}"
        );

        let remaining = crate::db::mappings::get_hash_ids(
            state.controller.db().db(),
            service.service_id,
            ContentStatus::Current,
            tag_id,
        )
        .unwrap();
        assert!(remaining.is_empty());

        let _ = std::fs::remove_dir_all(directory);
    }
}
