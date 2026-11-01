//! Client API access keys and their permissions.
//!
//! Permissions live in `api permissions.json` next to the database. The local
//! API server reads the same file, so saving an entry restarts the listener
//! through [`Controller::refresh_api`].

use std::collections::BTreeSet;

use serde_json::Value;

use crate::api::{load_permissions, ApiPermissions, ALLOWED_PERMISSIONS};

use super::{render, services, tag_filter, AppState, UiCommand};

/// R2: dialog spelling of a tag slice (`*` for all unnamespaced tags,
/// `*:*` for all namespaced ones, the slice otherwise). Mirrors the C++
/// `api_permission_slice_display` it replaces.
pub fn permission_slice_display(slice: &str) -> String {
    if slice.is_empty() {
        "*".to_string()
    } else if slice == ":" {
        "*:*".to_string()
    } else {
        slice.to_string()
    }
}

/// R2: permission summary of one API access key row.
pub fn permission_summary(entry: &ApiPermissions) -> String {
    if entry.permits_everything {
        return "permits everything".to_string();
    }

    let labels: Vec<&str> = ALLOWED_PERMISSIONS
        .iter()
        .filter(|(value, _)| entry.has_permission(*value))
        .map(|(_, label)| *label)
        .collect();

    if labels.is_empty() {
        "no permissions".to_string()
    } else {
        labels.join(", ")
    }
}

pub fn permissions_state(state: &AppState) -> crate::gui::qt::api::ffi::ApiPermissionsDialogState {
    use crate::gui::qt::api::ffi::{
        ApiAvailablePermission, ApiPermissionRow, ApiPermissionRule, ApiPermissionsDialogState,
    };

    let permissions = load_permissions(&state.controller.api_permissions_path());

    let rows: Vec<ApiPermissionRow> = permissions
        .iter()
        .map(|entry| {
            let rules: Vec<ApiPermissionRule> = entry
                .search_tag_filter
                .iter()
                .map(|(slice, rule)| ApiPermissionRule {
                    slice: slice.clone(),
                    display: permission_slice_display(slice),
                    rule: rule.clone(),
                })
                .collect();

            ApiPermissionRow {
                name: entry.name.clone(),
                access_key: entry.access_key.clone(),
                summary: permission_summary(entry),
                permits_everything: entry.permits_everything,
                basic_permissions: entry.basic_permissions.iter().map(|value| i32::from(*value)).collect(),
                rules,
                ..Default::default()
            }
        })
        .collect();

    let available: Vec<ApiAvailablePermission> = ALLOWED_PERMISSIONS
        .iter()
        .map(|(value, label)| ApiAvailablePermission {
            value: i32::from(*value),
            label: label.to_string(),
        })
        .collect();

    let api_status = state
        .controller
        .api_status()
        .unwrap_or_else(|| "stopped".to_string());

    ApiPermissionsDialogState {
        api_status: api_status.clone(),
        status: format!("api: {api_status}"),
        rows,
        available,
    }
}

pub fn open(state: &AppState) -> Vec<UiCommand> {
    vec![UiCommand::ApiPermissionsDialog(permissions_state(state))]
}

/// Typed reverse seam (WS-15.2.4c, партия 4): C++ передаёт basic
/// permissions списком, а tag filter — параллельными списками
/// `(slice, rule)` вместо JSON-payload.
pub fn save_typed(
    state: &mut AppState,
    name: &str,
    access_key: &str,
    original: &str,
    permits_everything: bool,
    basic: Vec<i32>,
    slices: Vec<String>,
    rules: Vec<String>,
) -> Vec<UiCommand> {
    let name = name.trim().to_string();

    if name.is_empty() {
        return notify(state, "the access key needs a name");
    }

    let access_key = access_key.trim().to_ascii_lowercase();

    if access_key.len() != 64
        || !access_key
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return notify(state, "the access key must be 64 hex characters");
    }

    let rules = match tag_filter::zip_rules_for_api(&slices, &rules) {
        Ok(rules) => rules,
        Err(error) => return notify(state, error),
    };

    save_core(
        state,
        name,
        access_key,
        original.trim().to_ascii_lowercase(),
        permits_everything,
        basic,
        rules,
    )
}

pub fn save(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return notify(state, "invalid API permissions payload");
    };

    let name = value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    if name.is_empty() {
        return notify(state, "the access key needs a name");
    }

    let access_key = value
        .get("access_key")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    if access_key.len() != 64
        || !access_key
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return notify(state, "the access key must be 64 hex characters");
    }

    let original = value
        .get("original_access_key")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let rules = match tag_filter::parse_rules(&value) {
        Ok(rules) => rules,
        Err(error) => return notify(state, error),
    };

    let mut basic = Vec::new();

    if let Some(values) = value.get("basic_permissions").and_then(Value::as_array) {
        for permission in values.iter().filter_map(Value::as_u64) {
            if let Ok(permission) = i32::try_from(permission) {
                basic.push(permission);
            }
        }
    }

    let permits_everything = value
        .get("permits_everything")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    save_typed(
        state,
        &name,
        &access_key,
        &original,
        permits_everything,
        basic,
        rules.iter().map(|(slice, _)| slice.clone()).collect(),
        rules.iter().map(|(_, rule)| rule.clone()).collect(),
    )
}

/// Чистый отбор basic permissions: только разрешённые, без дублей (R2).
pub fn filter_basic_permissions(chosen: &[i32]) -> Vec<u16> {
    let mut available: BTreeSet<u16> = ALLOWED_PERMISSIONS
        .iter()
        .map(|(value, _)| *value)
        .collect();
    let mut basic_permissions = Vec::new();

    for permission in chosen {
        if let Ok(permission) = u16::try_from(*permission) {
            if available.remove(&permission) {
                basic_permissions.push(permission);
            }
        }
    }

    basic_permissions
}

fn save_core(
    state: &mut AppState,
    name: String,
    access_key: String,
    original: String,
    permits_everything: bool,
    basic: Vec<i32>,
    rules: Vec<(String, String)>,
) -> Vec<UiCommand> {
    let basic_permissions = filter_basic_permissions(&basic);

    if !permits_everything && basic_permissions.is_empty() {
        return notify(state, "choose at least one permission");
    }

    let mut permissions = load_permissions(&state.controller.api_permissions_path());

    let entry = ApiPermissions {
        name: name.clone(),
        access_key: access_key.clone(),
        permits_everything,
        basic_permissions,
        search_tag_filter: rules,
    };

    let existing = if original.is_empty() {
        None
    } else {
        permissions
            .iter()
            .position(|entry| entry.access_key == original)
    };

    if let Some(index) = existing {
        if permissions
            .iter()
            .enumerate()
            .any(|(other_index, entry)| other_index != index && entry.access_key == access_key)
        {
            return notify(state, "another access key already uses that key");
        }

        permissions[index] = entry;
    } else {
        if permissions
            .iter()
            .any(|entry| entry.access_key == access_key)
        {
            return notify(state, "another access key already uses that key");
        }

        permissions.push(entry);
    }

    permissions.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    if let Err(error) = state.controller.save_api_permissions(&permissions) {
        return notify(state, format!("could not save the permissions: {error}"));
    }

    let api_status = state
        .controller
        .refresh_api()
        .unwrap_or_else(|error| format!("api failed: {error}"));

    updated(
        state,
        format!("permissions for '{name}' saved; {api_status}"),
    )
}

pub fn delete(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    // Строковый диспетчер (меню/палитра): полезной нагрузки нет, ключ едет
    // типизированно через `delete_typed`.
    delete_typed(state, payload)
}

pub fn delete_typed(state: &mut AppState, access_key: &str) -> Vec<UiCommand> {
    let access_key = access_key.trim().to_ascii_lowercase();
    let mut permissions = load_permissions(&state.controller.api_permissions_path());
    let before = permissions.len();
    permissions.retain(|entry| entry.access_key != access_key);

    if permissions.len() == before {
        return notify(state, "that access key is missing");
    }

    if let Err(error) = state.controller.save_api_permissions(&permissions) {
        return notify(state, format!("could not save the permissions: {error}"));
    }

    let api_status = state
        .controller
        .refresh_api()
        .unwrap_or_else(|error| format!("api failed: {error}"));

    updated(state, format!("access key deleted; {api_status}"))
}

fn updated(state: &AppState, message: impl Into<String>) -> Vec<UiCommand> {
    vec![
        UiCommand::UpdateApiPermissions(permissions_state(state)),
        UiCommand::Menus(render::menus(state)),
        UiCommand::Status(render::status(state)),
        UiCommand::NotifyMessage(message.into()),
    ]
}

fn notify(state: &AppState, message: impl Into<String>) -> Vec<UiCommand> {
    let mut commands = vec![UiCommand::NotifyMessage(message.into())];
    commands.extend([
        UiCommand::UpdateApiPermissions(permissions_state(state)),
        UiCommand::Menus(render::menus(state)),
        UiCommand::Status(render::status(state)),
    ]);
    commands
}

/// Local random 32-byte key as 64 hex characters, used by the editor.
pub fn new_access_key() -> String {
    use rand::RngCore;

    let mut key = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    services::hex_encode(&key)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::gui::app::AppState;

    fn test_state(directory: &std::path::Path) -> AppState {
        let _ = crate::client_options::register_serialisable_type();
        let controller = crate::controller::Controller::new(directory).unwrap();
        controller.init_model().unwrap();
        let mut options = controller.options();
        options.set_boolean("api_enabled", false);
        controller.replace_options(options);
        controller.save_options().unwrap();

        AppState::new(Arc::new(controller))
    }

    fn temp_directory(label: &str) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_api_permissions_gui_{label}_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    fn payload(name: &str, access_key: &str, original: &str) -> String {
        json!({
            "name": name,
            "access_key": access_key,
            "original_access_key": original,
            "permits_everything": false,
            "basic_permissions": [3, 2],
            "rules": [
                {"slice": ":", "rule": "blacklist"},
                {"slice": "character:", "rule": "whitelist"},
            ],
        })
        .to_string()
    }

    #[test]
    fn api_permission_cell_texts() {
        assert_eq!(permission_slice_display(""), "*");
        assert_eq!(permission_slice_display(":"), "*:*");
        assert_eq!(permission_slice_display("character:"), "character:");

        let everything = ApiPermissions {
            name: "all".to_string(),
            access_key: "k".to_string(),
            permits_everything: true,
            basic_permissions: vec![],
            search_tag_filter: vec![],
        };
        assert_eq!(permission_summary(&everything), "permits everything");

        let none = ApiPermissions {
            permits_everything: false,
            ..everything.clone()
        };
        assert_eq!(permission_summary(&none), "no permissions");

        let some = ApiPermissions {
            permits_everything: false,
            basic_permissions: vec![3, 2],
            ..everything.clone()
        };
        let summary = permission_summary(&some);
        let expected: Vec<&str> = ALLOWED_PERMISSIONS
            .iter()
            .filter(|(value, _)| *value == 3 || *value == 2)
            .map(|(_, label)| *label)
            .collect();
        assert_eq!(summary, expected.join(", "));
    }

    #[test]
    fn saving_and_deleting_an_access_key_persists_the_file() {
        let directory = temp_directory("save");
        let mut state = test_state(&directory);
        let key = new_access_key();

        let commands = save(&mut state, &payload("reader", &key, ""));
        assert!(
            commands.iter().any(|command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("saved"))),
            "{commands:?}"
        );

        let loaded = load_permissions(&state.controller.api_permissions_path());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "reader");
        assert_eq!(loaded[0].basic_permissions, vec![3, 2]);
        assert!(!loaded[0].permits_everything);

        let opened = open(&state);
        let name = match opened.first() {
            Some(UiCommand::ApiPermissionsDialog(state)) => state.rows[0].name.clone(),
            other => panic!("unexpected command: {other:?}"),
        };
        assert_eq!(name, "reader");

        delete(&mut state, &key);
        assert!(load_permissions(&state.controller.api_permissions_path()).is_empty());

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn duplicate_and_malformed_keys_are_rejected() {
        let directory = temp_directory("reject");
        let mut state = test_state(&directory);
        let key = new_access_key();

        save(&mut state, &payload("first", &key, ""));
        let commands = save(&mut state, &payload("second", &key, ""));
        assert!(
            commands.iter().any(|command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("already uses"))),
            "{commands:?}"
        );

        let commands = save(&mut state, &payload("bad", "not-hex", ""));
        assert!(
            commands.iter().any(|command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("64 hex"))),
            "{commands:?}"
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn a_key_can_be_renamed_or_rekeyed() {
        let directory = temp_directory("rekey");
        let mut state = test_state(&directory);
        let first = new_access_key();
        let second = new_access_key();

        save(&mut state, &payload("first", &first, ""));
        save(&mut state, &payload("renamed", &second, &first));

        let loaded = load_permissions(&state.controller.api_permissions_path());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "renamed");
        assert_eq!(loaded[0].access_key, second);

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn basic_permission_filter_keeps_only_known_once() {
        let first = ALLOWED_PERMISSIONS[0].0 as i32;
        assert_eq!(filter_basic_permissions(&[first, first, 9999]), vec![ALLOWED_PERMISSIONS[0].0]);
        assert!(filter_basic_permissions(&[]).is_empty());
    }
}
