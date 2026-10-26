//! Options, service manager and network manager payloads.
//!
//! The Qt layer renders these dictionaries as ordinary Hydrus-style dialogs;
//! all validation and persistence stays here.

use hydrus_core::hydrus_constants::ServiceType;
use serde_json::{Value, json};

use crate::client_options::ClientOptions;
use crate::gui::app::{AppState, UiCommand, render};
use crate::networking::client_networking_url_class::{PathComponent, UrlClass};

pub fn options_json(state: &AppState) -> String {
    let controller = &state.controller;
    let options = controller.options();

    let mut tabs = Vec::new();

    tabs.push(tab(
        "general",
        "general",
        vec![
            boolean(&options, "confirm_all_page_closes", "confirm closing pages"),
            boolean(
                &options,
                "set_search_focus_on_page_change",
                "focus the search box on page change",
            ),
            boolean(
                &options,
                "show_extended_single_file_info_in_status_bar",
                "show full file info in the status bar",
            ),
            boolean(&options, "always_loop_gifs", "loop gifs"),
            boolean(&options, "advanced_mode", "show advanced controls"),
            integer(
                &options,
                "max_page_name_chars",
                "maximum page name characters",
            ),
            integer(
                &options,
                "popup_message_character_width",
                "popup message width",
            ),
        ],
    ));

    tabs.push(tab(
        "files",
        "files and import",
        vec![
            boolean(&options, "use_qt_file_dialogs", "use Qt file dialogs"),
            boolean(
                &options,
                "file_viewing_statistics_active",
                "record file viewing statistics",
            ),
            integer(
                &options,
                "similar_files_duplicate_pairs_search_distance",
                "duplicate pair creation distance (0 = exact)",
            ),
            integer(&options, "thumbnail_border", "thumbnail border"),
            integer(&options, "thumbnail_margin", "thumbnail margin"),
            integer(&options, "thumbnail_cache_size", "thumbnail cache bytes"),
            integer(&options, "image_cache_size", "image cache bytes"),
        ],
    ));

    tabs.push(tab(
        "downloads",
        "downloaders",
        vec![
            integer(&options, "network_timeout", "network timeout (seconds)"),
            integer(
                &options,
                "watcher_check_period",
                "default watcher check period (seconds)",
            ),
            integer(
                &options,
                "subscription_check_period",
                "default subscription period (seconds)",
            ),
            string(
                &options,
                "favourite_simple_downloader_formula",
                "simple downloader formula name",
            ),
        ],
    ));

    tabs.push(tab(
        "network",
        "network",
        vec![
            noneable_string(&options, "http_proxy", "http proxy"),
            noneable_string(&options, "https_proxy", "https proxy"),
            noneable_string(&options, "no_proxy", "no proxy"),
            integer(&options, "max_network_jobs", "maximum network jobs"),
            integer(
                &options,
                "max_network_jobs_per_domain",
                "maximum jobs per domain",
            ),
            integer(
                &options,
                "max_connection_attempts_allowed",
                "maximum connection attempts",
            ),
        ],
    ));

    tabs.push(tab(
        "api",
        "client api",
        vec![
            boolean(&options, "api_enabled", "run the local Client API"),
            integer(&options, "api_port", "api port"),
            string(&options, "api_access_key", "access key (64 hex characters)"),
            str_field(
                "api_status",
                "api status",
                controller
                    .api_status()
                    .unwrap_or_else(|| "stopped".to_string()),
                "read only; changes apply immediately",
            ),
        ],
    ));

    json!({
        "tabs": tabs,
        "title": "options",
    })
    .to_string()
}

pub fn apply_options(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return vec![UiCommand::NotifyMessage(
            "invalid options payload".to_string(),
        )];
    };

    let mut options = state.controller.options();
    apply_maps(&mut options, &value);

    if options.get_boolean("api_enabled") && options.get_string("api_access_key").len() != 64 {
        options.set_string("api_access_key", &random_hex_32());
    }

    state.controller.replace_options(options);
    state.runtime.update_options(state.controller.options());

    if let Err(error) = state.controller.save_options() {
        state.message = format!("could not save options: {error}");
    } else {
        let api_status = state
            .controller
            .refresh_api()
            .unwrap_or_else(|error| format!("api failed: {error}"));
        state.message = format!("options saved; {api_status}");
    }

    let mut commands = vec![
        UiCommand::Menus(render::menus_json(state)),
        UiCommand::Status(render::status_json(state)),
        UiCommand::NotifyMessage(state.message.clone()),
    ];

    if state.controller.network_paused() {
        state.message = "network is paused".to_string();
        commands.push(UiCommand::Status(render::status_json(state)));
    }

    commands
}

pub fn services_json(state: &AppState) -> String {
    let manager = state.controller.services();

    let services: Vec<Value> = manager
        .get_services(None)
        .into_iter()
        .map(|service| {
            json!({
                "service_id": service.service_id,
                "key": hex(&service.service_key),
                "name": service.name,
                "type": service.service_type.as_str(),
                "type_short": service.service_type.short_str(),
                "type_int": service.service_type.as_int(),
                "builtin": matches!(service.name.as_str(), "my files" | "local tags" | "trash"),
            })
        })
        .collect();

    json!({
        "services": services,
        "types": [
            {"label": "local file domain", "value": ServiceType::LocalFileDomain.as_int(), "short": "local file domain"},
            {"label": "local tag domain", "value": ServiceType::LocalTag.as_int(), "short": "local tag domain"},
            {"label": "local notes", "value": ServiceType::LocalNotes.as_int(), "short": "local file notes service"},
            {"label": "inc/dec ratings", "value": ServiceType::LocalRatingIncDec.as_int(), "short": "local inc/dec rating service"}
        ]
    })
    .to_string()
}

pub fn service_action(state: &mut AppState, action: &str, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        state.message = "invalid service payload".to_string();
        return service_commands(state);
    };

    let result = match action {
        "service_add" => {
            let name = value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let service_type = value
                .get("type")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok())
                .and_then(ServiceType::try_from_int);

            match service_type {
                Some(service_type) => state
                    .controller
                    .add_service(name, service_type)
                    .map(|_| format!("service '{name}' created")),
                None => Err(hydrus_core::hydrus_exceptions::unknown(
                    "unknown service type",
                )),
            }
        }
        "service_rename" => {
            let key = value.get("key").and_then(Value::as_str).unwrap_or_default();
            let name = value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();

            state
                .controller
                .rename_service(key, name)
                .map(|_| format!("service renamed to '{name}'"))
        }
        "service_delete" => {
            let key = value.get("key").and_then(Value::as_str).unwrap_or_default();

            state
                .controller
                .delete_service(key)
                .map(|_| "service deleted".to_string())
        }
        _ => Err(hydrus_core::hydrus_exceptions::unknown(
            "unknown service action",
        )),
    };

    state.message = match result {
        Ok(message) => message,
        Err(error) => format!("service error: {error}"),
    };

    if state.controller.api_status().is_some() {
        let _ = state.controller.refresh_api();
    }

    service_commands(state)
}

pub fn network_json(state: &AppState) -> String {
    let controller = &state.controller;
    let options = controller.options();
    let domain_manager = controller.domain_manager().read().unwrap();

    let headers: Vec<Value> = domain_manager
        .get_custom_headers()
        .iter()
        .map(|(name, header)| {
            json!({
                "name": name,
                "value": header.value,
                "approved": header.approved,
                "note": header.note,
            })
        })
        .collect();

    let url_classes: Vec<Value> = domain_manager
        .get_url_classes()
        .iter()
        .map(|class| {
            json!({
                "name": class.name,
                "netloc": class.netloc,
                "components": class.path_components.iter().map(|component| match component {
                    PathComponent::Fixed(value) => value.clone(),
                    PathComponent::Variable { name, .. } => format!("{{{name}}}"),
                }).collect::<Vec<_>>(),
            })
        })
        .collect();

    json!({
        "options": {
            "http_proxy": options.get_noneable_string("http_proxy"),
            "https_proxy": options.get_noneable_string("https_proxy"),
            "no_proxy": options.get_noneable_string("no_proxy"),
            "network_timeout": options.get_integer("network_timeout"),
            "max_network_jobs": options.get_integer("max_network_jobs"),
            "max_network_jobs_per_domain": options.get_integer("max_network_jobs_per_domain"),
            "api_enabled": options.get_boolean("api_enabled"),
            "api_port": options.get_integer("api_port"),
            "api_access_key": options.get_string("api_access_key"),
        },
        "paused": controller.network_paused(),
        "api_status": controller.api_status().unwrap_or_else(|| "stopped".to_string()),
        "headers": headers,
        "url_classes": url_classes,
    })
    .to_string()
}

pub fn network_action(state: &mut AppState, action: &str, payload: &str) -> Vec<UiCommand> {
    match action {
        "network_save" => {
            let Ok(value) = serde_json::from_str::<Value>(payload) else {
                state.message = "invalid network payload".to_string();
                return network_commands(state);
            };

            let mut options = state.controller.options();
            apply_maps(&mut options, &value);

            if options.get_boolean("api_enabled")
                && options.get_string("api_access_key").len() != 64
            {
                options.set_string("api_access_key", &random_hex_32());
            }

            state.controller.replace_options(options);
            state.runtime.update_options(state.controller.options());
            let _ = state.controller.save_options();
            let status = state
                .controller
                .refresh_api()
                .unwrap_or_else(|error| format!("api failed: {error}"));

            state.message = format!("network settings saved; {status}");
        }
        "network_pause" => {
            let paused = !state.controller.network_paused();
            state.controller.set_network_paused(paused);
            state.message = if paused {
                "all new network traffic is paused".to_string()
            } else {
                "network traffic resumed".to_string()
            };
        }
        "network_header_add" | "network_header_remove" => {
            let Ok(value) = serde_json::from_str::<Value>(payload) else {
                state.message = "invalid header payload".to_string();
                return network_commands(state);
            };

            let name = value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();

            if name.is_empty() {
                state.message = "a header needs a name".to_string();
                return network_commands(state);
            }

            let mut manager = state.controller.domain_manager().write().unwrap();

            if action == "network_header_remove" {
                manager.remove_custom_header(&name);
                state.message = format!("removed header '{name}'");
            } else {
                let value = value
                    .get("value")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                manager.set_custom_header(
                    name.clone(),
                    crate::client_defaults::CustomHeader {
                        value: value.to_string(),
                        approved: true,
                        note: "added in the network manager".to_string(),
                    },
                );
                state.message = format!("saved header '{name}'");
            }
        }
        "network_url_class_add" => {
            let Ok(value) = serde_json::from_str::<Value>(payload) else {
                state.message = "invalid url class payload".to_string();
                return network_commands(state);
            };

            let name = value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("url class");
            let netloc = value
                .get("netloc")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let path = value
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();

            let mut class = UrlClass::new(name, netloc);

            for segment in path.split('/').filter(|segment| !segment.is_empty()) {
                if segment.starts_with('{') && segment.ends_with('}') {
                    class = class.with_variable(segment.trim_matches(['{', '}']));
                } else {
                    class
                        .path_components
                        .push(PathComponent::Fixed(segment.to_string()));
                }
            }

            state
                .controller
                .domain_manager()
                .write()
                .unwrap()
                .add_url_class(class);
            state.message = format!("url class '{name}' added");
        }
        "network_url_class_remove" => {
            let Ok(value) = serde_json::from_str::<Value>(payload) else {
                state.message = "invalid url class payload".to_string();
                return network_commands(state);
            };

            let name = value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();

            let removed = state
                .controller
                .domain_manager()
                .write()
                .unwrap()
                .remove_url_class(name);
            state.message = if removed {
                format!("url class '{name}' removed")
            } else {
                format!("url class '{name}' was not found")
            };
        }
        _ => {
            state.message = format!("unknown network action: {action}");
        }
    }

    let _ = state.controller.save_network_settings();

    let headers = state
        .controller
        .domain_manager()
        .read()
        .map(|manager| manager.get_custom_header_pairs())
        .unwrap_or_default();
    state.runtime.update_network_headers(headers);

    network_commands(state)
}

fn network_commands(state: &AppState) -> Vec<UiCommand> {
    vec![
        UiCommand::NetworkDialog(network_json(state)),
        UiCommand::Menus(render::menus_json(state)),
        UiCommand::Status(render::status_json(state)),
        UiCommand::NotifyMessage(state.message.clone()),
    ]
}

fn service_commands(state: &AppState) -> Vec<UiCommand> {
    vec![
        UiCommand::ServicesDialog(services_json(state)),
        UiCommand::Menus(render::menus_json(state)),
        UiCommand::Status(render::status_json(state)),
        UiCommand::NotifyMessage(state.message.clone()),
    ]
}

fn apply_maps(options: &mut ClientOptions, value: &Value) {
    if let Some(map) = value.get("booleans").and_then(Value::as_object) {
        for (key, value) in map {
            if let Some(value) = value.as_bool() {
                options.set_boolean(key, value);
            }
        }
    }

    if let Some(map) = value.get("integers").and_then(Value::as_object) {
        for (key, value) in map {
            if let Some(value) = value.as_i64() {
                options.set_integer(key, value);
            }
        }
    }

    if let Some(map) = value.get("strings").and_then(Value::as_object) {
        for (key, value) in map {
            if let Some(value) = value.as_str() {
                options.set_string(key, value);
            }
        }
    }

    if let Some(map) = value.get("noneable_strings").and_then(Value::as_object) {
        for (key, value) in map {
            let value = value.as_str().filter(|value| !value.trim().is_empty());
            options.set_noneable_string(key, value);
        }
    }

    if let Some(map) = value.get("noneable_integers").and_then(Value::as_object) {
        for (key, value) in map {
            options.set_noneable_integer(key, value.as_i64());
        }
    }
}

fn tab(name: &str, title: &str, fields: Vec<Value>) -> Value {
    json!({"name": name, "title": title, "fields": fields})
}

fn boolean(options: &ClientOptions, key: &str, label: &str) -> Value {
    json!({
        "key": key,
        "label": label,
        "type": "bool",
        "value": options.get_boolean(key),
    })
}

fn integer(options: &ClientOptions, key: &str, label: &str) -> Value {
    json!({
        "key": key,
        "label": label,
        "type": "int",
        "value": options.get_integer(key),
    })
}

fn string(options: &ClientOptions, key: &str, label: &str) -> Value {
    json!({
        "key": key,
        "label": label,
        "type": "string",
        "value": options.get_string(key),
    })
}

fn noneable_string(options: &ClientOptions, key: &str, label: &str) -> Value {
    json!({
        "key": key,
        "label": label,
        "type": "string",
        "noneable": true,
        "value": options.get_noneable_string(key).unwrap_or_default(),
    })
}

fn str_field(key: &str, label: &str, value: String, note: &str) -> Value {
    json!({
        "key": key,
        "label": label,
        "type": "readonly",
        "value": value,
        "note": note,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn random_hex_32() -> String {
    use rand::RngCore;

    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
