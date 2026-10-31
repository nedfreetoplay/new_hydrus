//! Import options editor and manager persistence.
//!
//! The dialog edits one container at a time (a caller default or a named
//! favourite) and sends the whole manager back on save. Stored under the
//! `import_options` options group, next to the other serialisable options.

use hydrus_core::hydrus_constants::ServiceType;
use serde_json::{Value, json};

use crate::controller::Controller;
use crate::importing::ImportOptionsContainer;
use crate::importing::options::{
    CALLER_TYPES, IMPORT_OPTIONS_TYPE_DESCRIPTIONS, IMPORT_OPTIONS_TYPE_LABELS, PREFETCH_DO_CHECK,
    PREFETCH_DO_CHECK_AND_MATCHES_ARE_DISPOSITIVE, PREFETCH_DO_NOT_CHECK,
    PRESENTATION_INBOX_AGNOSTIC, PRESENTATION_INBOX_AND_INCLUDE_ALL_INBOX,
    PRESENTATION_INBOX_REQUIRE_INBOX, PRESENTATION_STATUS_ANY_GOOD, PRESENTATION_STATUS_NEW_ONLY,
    PRESENTATION_STATUS_NONE, file_type_classes, mime_class_extensions, option_types_for_caller,
};
use crate::importing::options_manager::ImportOptionsManager;

use super::{AppState, UiCommand, query, render};

pub fn load(controller: &Controller) -> ImportOptionsManager {
    ImportOptionsManager::load_from_options(&controller.options())
}

pub fn save_manager(controller: &Controller, manager: &ImportOptionsManager) {
    let mut options = controller.options();
    manager.save_to_options(&mut options);
    controller.replace_options(options);
    let _ = controller.save_options();
}

/// The options used by the plain local import pipeline.
pub fn local_import_options(controller: &Controller) -> ImportOptionsContainer {
    load(controller).get_effective("local_import", None)
}

/// Resolves a per-page/subscription override against its caller default and
/// global options.
pub fn effective_for(
    controller: &Controller,
    caller: &str,
    specific: &ImportOptionsContainer,
) -> ImportOptionsContainer {
    let specific = (!specific.is_empty()).then_some(specific);

    load(controller).get_effective(caller, specific)
}

pub fn dialog_json(state: &AppState) -> String {
    let manager = load(&state.controller);
    let services = state.controller.services();
    let tag_services: Vec<Value> = services
        .get_services(Some(&[ServiceType::LocalTag, ServiceType::TagRepository]))
        .into_iter()
        .map(|service| {
            json!({
                "key": query::hex(&service.service_key),
                "name": service.name,
            })
        })
        .collect();
    let file_services: Vec<String> = services
        .get_services(Some(&[ServiceType::LocalFileDomain]))
        .into_iter()
        .map(|service| service.name.clone())
        .collect();

    let callers = CALLER_TYPES
        .iter()
        .map(|(key, label, description)| {
            json!({
                "key": key,
                "label": label,
                "description": description,
                "types": option_types_for_caller(key),
            })
        })
        .collect::<Vec<_>>();

    let types = IMPORT_OPTIONS_TYPE_LABELS
        .iter()
        .map(|(key, label)| {
            json!({
                "key": key,
                "label": label,
                "description": IMPORT_OPTIONS_TYPE_DESCRIPTIONS
                    .iter()
                    .find(|(type_key, _)| type_key == key)
                    .map(|(_, description)| *description)
                    .unwrap_or(""),
            })
        })
        .collect::<Vec<_>>();

    let file_types = file_type_classes()
        .into_iter()
        .map(|mime| {
            json!({
                "value": mime.as_str(),
                "label": mime.as_str(),
                "extensions": mime_class_extensions(mime),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "caller_types": callers,
        "types": types,
        "defaults": manager.get_defaults(),
        "favourites": manager.get_favourites(),
        "tag_services": tag_services,
        "file_services": file_services,
        "file_types": file_types,
        "prefetch_check_types": [
            {"value": PREFETCH_DO_NOT_CHECK, "label": "do not check"},
            {"value": PREFETCH_DO_CHECK, "label": "check"},
            {"value": PREFETCH_DO_CHECK_AND_MATCHES_ARE_DISPOSITIVE, "label": "check, matches are final"},
        ],
        "note_conflicts": [
            {"value": "replace", "label": "replace the existing note"},
            {"value": "ignore", "label": "do not add the new note"},
            {"value": "append", "label": "append to the existing note"},
            {"value": "rename", "label": "add under a new name"},
        ],
        "presentation_statuses": [
            {"value": PRESENTATION_STATUS_ANY_GOOD, "label": "all files"},
            {"value": PRESENTATION_STATUS_NEW_ONLY, "label": "new files"},
            {"value": PRESENTATION_STATUS_NONE, "label": "do not show anything"},
        ],
        "presentation_inboxes": [
            {"value": PRESENTATION_INBOX_AGNOSTIC, "label": "inbox or archive"},
            {"value": PRESENTATION_INBOX_REQUIRE_INBOX, "label": "must be in inbox"},
            {"value": PRESENTATION_INBOX_AND_INCLUDE_ALL_INBOX, "label": "or in inbox"},
        ],
        "rules": [
            {"value": "blacklist", "label": "blacklist"},
            {"value": "whitelist", "label": "whitelist"},
        ],
    })
    .to_string()
}

pub fn open(state: &AppState) -> Vec<UiCommand> {
    vec![UiCommand::ImportOptionsDialog(dialog_json(state))]
}

/// Opens the options dialog with one caller row selected, used by the local
/// import review panel.
pub fn open_caller(state: &AppState, caller: &str) -> Vec<UiCommand> {
    let mut value: Value = serde_json::from_str(&dialog_json(state)).unwrap_or_else(|_| json!({}));

    if value.is_object() {
        value["selected"] = json!(format!("default:{caller}"));
    }

    vec![UiCommand::ImportOptionsDialog(value.to_string())]
}

/// Builds the editor for a specific importer container. The container only
/// stores overrides; unchecked option tabs inherit the caller default and then
/// global options.
pub fn specific_dialog_json(
    state: &AppState,
    caller: &str,
    container: &ImportOptionsContainer,
    target: &str,
    action: &str,
) -> String {
    let mut value: Value = serde_json::from_str(&dialog_json(state)).unwrap_or_else(|_| json!({}));
    let fallback = load(&state.controller).get_effective(caller, None);

    if let Some(object) = value.as_object_mut() {
        object.insert(
            "specific".to_string(),
            json!({
                "caller": caller,
                "target": target,
                "action": action,
                "container": container,
            }),
        );
        object.insert("fallback".to_string(), json!(fallback));
    }

    value.to_string()
}

pub fn specific_open(
    state: &AppState,
    caller: &str,
    container: &ImportOptionsContainer,
    target: &str,
    action: &str,
) -> Vec<UiCommand> {
    vec![UiCommand::ImportOptionsDialog(specific_dialog_json(
        state, caller, container, target, action,
    ))]
}

pub fn save_action(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return vec![UiCommand::NotifyMessage(
            "could not read the import options".to_string(),
        )];
    };

    let mut manager = match ImportOptionsManager::from_json(value) {
        Ok(manager) => manager,
        Err(error) => {
            return vec![UiCommand::NotifyMessage(format!(
                "could not read the import options: {error}"
            ))];
        }
    };

    manager.ensure_defaults();
    save_manager(&state.controller, &manager);
    state.runtime.update_options(state.controller.options());
    state.message = "import options saved".to_string();

    vec![
        UiCommand::NotifyMessage("import options saved".to_string()),
        UiCommand::Status(render::status(state)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state(directory: &std::path::Path) -> AppState {
        let _ = crate::client_options::register_serialisable_type();
        let controller = Controller::new(directory).unwrap();
        controller.init_model().unwrap();

        AppState::new(std::sync::Arc::new(controller))
    }

    #[test]
    fn dialog_exposes_containers_types_and_services() {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_import_options_test_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let state = test_state(&directory);

        let value: Value = serde_json::from_str(&dialog_json(&state)).unwrap();

        assert_eq!(value["caller_types"].as_array().unwrap().len(), 9);
        assert!(
            value["defaults"]["global"]["file_filtering"].is_object(),
            "global defaults are fully populated"
        );
        assert!(value["favourites"].as_object().unwrap().is_empty());
        assert!(!value["file_types"].as_array().unwrap().is_empty());
        assert!(!value["note_conflicts"].as_array().unwrap().is_empty());

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn saving_the_dialog_payload_persists_favourites() {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_import_options_save_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let mut state = test_state(&directory);
        let dialog: Value = serde_json::from_str(&dialog_json(&state)).unwrap();

        let payload = json!({
            "defaults": dialog["defaults"],
            "favourites": {
                "quiet": {
                    "presentation": {
                        "presentation_status": "new",
                        "presentation_inbox": "agnostic"
                    }
                }
            }
        })
        .to_string();

        let commands = save_action(&mut state, &payload);

        assert!(matches!(
            commands.first(),
            Some(UiCommand::NotifyMessage(_))
        ));

        let manager = load(&state.controller);
        let quiet = manager.get_favourite("quiet").unwrap();
        assert_eq!(
            quiet.presentation.as_ref().unwrap().presentation_status,
            "new"
        );
        assert!(manager.get_default("global").is_full());

        let _ = std::fs::remove_dir_all(directory);
    }
}
