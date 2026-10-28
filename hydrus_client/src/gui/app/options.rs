//! Options, service manager and network manager payloads.
//!
//! The Qt layer renders these dictionaries as ordinary Hydrus-style dialogs;
//! all validation and persistence stays here.

use serde_json::{Value, json};

use crate::client_options::ClientOptions;
use crate::gui::app::{AppState, UiCommand, render};
use crate::networking::client_networking_url_class::PathComponent;

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
        "notifications",
        "popup notifications",
        vec![
            integer(
                &options,
                "popup_message_character_width",
                "popup message width (characters)",
            ),
            integer(
                &options,
                "popup_message_max_messages",
                "messages kept in the popup history",
            ),
        ],
    ));

    tabs.push(tab(
        "statistics",
        "file viewing statistics",
        vec![
            boolean(
                &options,
                "file_viewing_statistics_active",
                "record file viewing statistics",
            ),
            str_field(
                "viewing_stats_note",
                "storage",
                "views and viewtime are written to file_viewing_stats".to_string(),
                "the viewer records media views; previews and the API use the same table",
            ),
        ],
    ));

    tabs.push(tab(
        "files",
        "files and import",
        vec![
            boolean(&options, "use_qt_file_dialogs", "use Qt file dialogs"),
            integer(
                &options,
                "similar_files_duplicate_pairs_search_distance",
                "duplicate pair creation distance (0 = exact)",
            ),
            boolean(
                &options,
                "duplicates_auto_resolution_enabled",
                "apply duplicate auto-resolution rules",
            ),
            integer(&options, "thumbnail_border", "thumbnail border"),
            integer(&options, "thumbnail_margin", "thumbnail margin"),
            integer(&options, "thumbnail_cache_size", "thumbnail cache bytes"),
            integer(&options, "image_cache_size", "image cache bytes"),
        ],
    ));

    tabs.push(tab(
        "exporting",
        "exporting",
        vec![
            boolean(
                &options,
                "pause_export_folders_sync",
                "pause export folder synchronisation",
            ),
            string(
                &options,
                "export_phrase",
                "default filename phrase",
            ),
            noneable_string(&options, "export_path", "default export directory"),
            integer(
                &options,
                "export_filename_character_limit",
                "filename character limit (0 = 255)",
            ),
            boolean(
                &options,
                "always_apply_ntfs_export_filename_rules",
                "always apply NTFS filename rules",
            ),
            str_field(
                "export_phrase_help",
                "phrase syntax",
                "{hash} {file_id} {#} {tags} {nn tags} [namespace] (tag)".to_string(),
                "the filename phrase is used by new export folders; tags become filename parts",
            ),
        ],
    ));

    tabs.push(tab(
        "tags",
        "tag presentation",
        vec![
            boolean(&options, "show_namespaces", "show namespaces"),
            boolean(
                &options,
                "replace_tag_underscores_with_spaces",
                "show underscores as spaces",
            ),
            boolean(
                &options,
                "expand_parents_on_storage_taglists",
                "expand parent tags on storage lists",
            ),
            string(
                &options,
                "tag_display_sort",
                "display sort (lexicographic/namespace/subtag)",
            ),
        ],
    ));

    tabs.push(tab(
        "thumbnails",
        "thumbnails",
        vec![
            integer(
                &options,
                "thumbnail_scale_percent",
                "thumbnail scale (percent)",
            ),
            boolean(
                &options,
                "thumbnail_layout_masonry",
                "masonry layout (variable-height rows)",
            ),
            boolean(
                &options,
                "thumbnail_scroll_horizontal",
                "scroll thumbnails horizontally",
            ),
            integer(&options, "thumbnail_border", "thumbnail border"),
            integer(&options, "thumbnail_margin", "thumbnail margin"),
        ],
    ));

    tabs.push(tab(
        "media",
        "media viewer",
        vec![
            boolean(
                &options,
                "media_viewer_window_always_on_top",
                "keep media viewers always on top",
            ),
            boolean(
                &options,
                "always_start_media_viewers_always_on_top",
                "start every viewer always on top",
            ),
            boolean(
                &options,
                "always_start_media_viewers_frameless",
                "start every viewer frameless",
            ),
            boolean(
                &options,
                "media_viewer_recenter_media_on_window_resize",
                "re-center media when the viewer is resized",
            ),
            boolean(
                &options,
                "media_viewer_lock_current_zoom_type",
                "lock the current zoom type",
            ),
            boolean(
                &options,
                "save_media_viewer_window_size_and_position_on_close",
                "remember viewer size and position",
            ),
            boolean(&options, "media_viewer_audio_mute", "mute viewer audio"),
            integer(
                &options,
                "media_viewer_zoom_step_percent",
                "zoom step (percent)",
            ),
            integer(
                &options,
                "media_viewer_prefetch_num_previous",
                "prefetch previous files",
            ),
            integer(
                &options,
                "media_viewer_prefetch_num_next",
                "prefetch next files",
            ),
            integer(
                &options,
                "media_viewer_audio_volume",
                "viewer audio volume (0-100)",
            ),
            str_field(
                "media_viewer_zoom_center",
                "zoom center",
                options.get_string("media_viewer_zoom_center"),
                "mouse or canvas; used by the viewer zoom shortcut",
            ),
        ],
    ));

    tabs.push(tab(
        "maintenance",
        "file maintenance",
        vec![
            boolean(
                &options,
                "file_maintenance_during_idle",
                "run file maintenance while idle",
            ),
            boolean(
                &options,
                "file_maintenance_during_active",
                "run file maintenance during normal use",
            ),
            boolean(
                &options,
                "database_deferred_delete_maintenance_during_idle",
                "drop deferred database tables while idle",
            ),
            boolean(
                &options,
                "database_deferred_delete_maintenance_during_active",
                "drop deferred database tables during normal use",
            ),
            boolean(
                &options,
                "maintain_trash_in_normal_time",
                "maintain the trash in normal time",
            ),
            boolean(
                &options,
                "deferred_file_deletes_in_normal_time",
                "defer file deletes in normal time",
            ),
            boolean(
                &options,
                "use_advanced_file_deletion_dialog",
                "use the advanced file deletion dialog",
            ),
            integer(
                &options,
                "thumbnail_cache_timeout",
                "thumbnail cache timeout (seconds)",
            ),
            integer(
                &options,
                "image_cache_timeout",
                "image cache timeout (seconds)",
            ),
            integer(
                &options,
                "shutdown_work_period",
                "shutdown work period (seconds)",
            ),
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
            str_field(
                "api_permissions",
                "access keys",
                format!("{} key(s)", controller.api_permissions().len()),
                "manage them from services > client api permissions...",
            ),
            string(
                &options,
                "api_access_key",
                "legacy access key (used only until permissions are saved)",
            ),
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
    state
        .runtime
        .update_pools(state.controller.download_pool_settings());

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
                "url_type": class.url_type,
                "url_type_name": crate::networking::client_networking_url_class::url_type_name(class.url_type),
                "preferred_scheme": class.preferred_scheme.clone().unwrap_or_default(),
                "netloc": class.netloc,
                "match_subdomains": class.match_subdomains,
                "keep_matched_subdomains": class.keep_matched_subdomains,
                "components": class.path_components.iter().map(|component| match component {
                    PathComponent::Fixed(value) => value.clone(),
                    PathComponent::Variable { name, .. } => format!("{{{name}}}"),
                }).collect::<Vec<_>>(),
                "parameters": class.parameters.iter().map(|(name, value)| match value {
                    Some(value) => format!("{name}={value}"),
                    None => name.clone(),
                }).collect::<Vec<_>>().join("\n"),
                "can_produce_multiple_files": class.can_produce_multiple_files,
                "should_be_associated_with_files": class.should_be_associated_with_files,
                "example_url": class.example_url,
            })
        })
        .collect();

    let pools = controller.download_pool_settings();
    let domain_limits: Vec<Value> = pools
        .domain_limits
        .iter()
        .map(|(domain, limit)| json!({"domain": domain, "limit": limit}))
        .collect();
    let bandwidth_rules = controller.bandwidth_rule_entries();

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
        "url_types": [
            {"value": 0, "label": "post url"},
            {"value": 1, "label": "api/redirect url"},
            {"value": 2, "label": "file url"},
            {"value": 3, "label": "gallery url"},
            {"value": 4, "label": "watchable url"},
            {"value": 5, "label": "unknown url"},
            {"value": 7, "label": "downloadable/pursuable url"},
            {"value": 9, "label": "sub-gallery url"}
        ],
        "download_pools": domain_limits,
        "bandwidth_rules": bandwidth_rules,
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

            if let Some(pools) = value.get("download_pools") {
                if let Ok(pools) = serde_json::from_value::<
                    crate::networking::download_pools::DownloadPoolSettings,
                >(pools.clone())
                {
                    state.controller.replace_download_pool_settings(pools);
                }
            }

            if let Some(rules) = value.get("bandwidth_rules") {
                if let Ok(rules) = serde_json::from_value::<
                    Vec<crate::networking::client_networking_bandwidth::BandwidthRuleEntry>,
                >(rules.clone())
                {
                    state.controller.replace_bandwidth_rule_entries(rules);
                }
            }

            state
                .runtime
                .update_pools(state.controller.download_pool_settings());

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
        "network_url_class_add" | "network_url_class_save" => {
            let Ok(value) = serde_json::from_str::<Value>(payload) else {
                state.message = "invalid url class payload".to_string();
                return network_commands(state);
            };

            let original = value
                .get("original")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let existing = state
                .controller
                .domain_manager()
                .read()
                .unwrap()
                .get_url_classes()
                .iter()
                .find(|class| class.name == original)
                .cloned();

            let Some(class) =
                crate::gui::app::metadata::url_class_from_json(&value, existing.as_ref())
            else {
                state.message = "a url class needs a name and a domain".to_string();
                return network_commands(state);
            };

            let name = class.name.clone();
            let mut manager = state.controller.domain_manager().write().unwrap();

            if !original.is_empty() && original != name {
                manager.remove_url_class(original);
            } else if action == "network_url_class_add" {
                manager.remove_url_class(&name);
            }

            manager.add_url_class(class);
            state.message = format!("url class '{name}' saved");
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
        "network_pool_set" => {
            let Ok(value) = serde_json::from_str::<Value>(payload) else {
                state.message = "invalid pool payload".to_string();
                return network_commands(state);
            };

            let domain = value
                .get("domain")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let limit = value.get("limit").and_then(Value::as_u64).unwrap_or(1) as usize;
            let mut pools = state.controller.download_pool_settings();
            pools.set_limit(domain, limit);
            state
                .controller
                .replace_download_pool_settings(pools.clone());
            state.runtime.update_pools(pools);
            state.message = format!("download pool for '{domain}' set to {limit}");
        }
        "network_pool_remove" => {
            let Ok(value) = serde_json::from_str::<Value>(payload) else {
                state.message = "invalid pool payload".to_string();
                return network_commands(state);
            };

            let domain = value
                .get("domain")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let mut pools = state.controller.download_pool_settings();
            pools.remove_limit(domain);
            state
                .controller
                .replace_download_pool_settings(pools.clone());
            state.runtime.update_pools(pools);
            state.message = format!("download pool for '{domain}' removed");
        }
        "network_rule_set" => {
            let Ok(value) = serde_json::from_str::<Value>(payload) else {
                state.message = "invalid bandwidth rule".to_string();
                return network_commands(state);
            };

            let Ok(entry) = serde_json::from_value::<
                crate::networking::client_networking_bandwidth::BandwidthRuleEntry,
            >(value) else {
                state.message = "invalid bandwidth rule".to_string();
                return network_commands(state);
            };

            let mut rules = state.controller.bandwidth_rule_entries();
            rules.retain(|existing| {
                !(existing.context_type == entry.context_type
                    && existing.context == entry.context
                    && existing.bandwidth_type == entry.bandwidth_type)
            });
            rules.push(entry);
            state.controller.replace_bandwidth_rule_entries(rules);
            state.message = "bandwidth rule saved".to_string();
        }
        "network_rule_remove" => {
            let Ok(value) = serde_json::from_str::<Value>(payload) else {
                state.message = "invalid bandwidth rule".to_string();
                return network_commands(state);
            };

            let context_type = value
                .get("context_type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let context = value
                .get("context")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let bandwidth_type = value
                .get("bandwidth_type")
                .and_then(Value::as_str)
                .unwrap_or_default();

            let mut rules = state.controller.bandwidth_rule_entries();
            let before = rules.len();
            rules.retain(|existing| {
                !(existing.context_type == context_type
                    && existing.context == context
                    && existing.bandwidth_type == bandwidth_type)
            });
            state.controller.replace_bandwidth_rule_entries(rules);
            state.message = if before > 0 {
                "bandwidth rule removed".to_string()
            } else {
                "bandwidth rule was not found".to_string()
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

fn random_hex_32() -> String {
    use rand::RngCore;

    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
