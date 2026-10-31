//! Options, service manager and network manager payloads.
//!
//! The Qt layer renders these dictionaries as ordinary Hydrus-style dialogs;
//! all validation and persistence stays here.

use serde_json::{Value, json};

use crate::client_options::ClientOptions;
use crate::gui::app::{AppState, UiCommand, render};
use crate::networking::client_networking_url_class::PathComponent;

#[derive(Debug, Clone, Copy)]
pub struct GeneralField {
    pub key: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub kind: &'static str,
}

pub static GENERAL_FIELDS: &[GeneralField] = &[
    GeneralField {
        key: "confirm_all_page_closes",
        label: "confirm closing pages",
        help: "With this, you will always be asked, even on single page closures of simple file pages.",
        kind: "bool",
    },
    GeneralField {
        key: "set_search_focus_on_page_change",
        label: "focus the search box on page change",
        help: "Set it so whenever you switch between pages, the keyboard focus immediately moves to the tag autocomplete or search text input.",
        kind: "bool",
    },
    GeneralField {
        key: "show_extended_single_file_info_in_status_bar",
        label: "show full file info in the status bar",
        help: "This will show, any time you have a single thumbnail selected, the file info summary you normally see in the top hover window of the media viewer in the main gui status bar. Check the \"media viewer hovers\" options panel to edit what this summary includes.",
        kind: "bool",
    },
    GeneralField {
        key: "always_loop_gifs",
        label: "loop gifs",
        help: "Some GIFS and APNGs have metadata specifying how many times they should be played, usually 1. Uncheck this to obey that number.",
        kind: "bool",
    },
    GeneralField {
        key: "advanced_mode",
        label: "show advanced controls",
        help: "This controls a variety of different features across the program, too many to list neatly. The plan is to blow this single option out into many granular options on this page.\n\nThis plan is failing!",
        kind: "bool",
    },
    GeneralField {
        key: "max_page_name_chars",
        label: "maximum page name characters",
        help: "Max characters to display in a page name.",
        kind: "int",
    },
    GeneralField {
        key: "popup_message_character_width",
        label: "popup message width",
        help: "Approximate max width of popup messages (in characters).",
        kind: "int",
    },
    GeneralField {
        key: "remember_options_window_panel",
        label: "remember last open options panel",
        help: "This will cause the options window (this one) to reopen at the last panel you were looking at when it was closed.",
        kind: "bool",
    },
    GeneralField {
        key: "options_search_bar_top_of_window",
        label: "options search bar at top of window",
        help: "Relocate the built in options search bar. Requires applying and re-opening this dialog to update.",
        kind: "bool",
    },
];

#[derive(Debug, Clone, Copy)]
pub struct DuplicateField {
    pub key: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub kind: &'static str,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub noneable: bool,
    pub none_phrase: &'static str,
    pub readonly_value: &'static str,
}

pub fn general_field(key: &str) -> Option<&'static GeneralField> {
    GENERAL_FIELDS.iter().find(|field| field.key == key)
}

pub static DUPLICATE_FIELDS: &[DuplicateField] = &[
    DuplicateField {
        key: "open_files_to_duplicate_filter_uses_all_my_files",
        label: "Set to \"combined local file domains\" when hitting \"Open files in a new duplicates filter page\":",
        help: "Normally, when you open a selection of files into a new page, the current file domain is preserved. For duplicates filters, you usually want to search in \"combined local file domains\", so this sticks that. If you need domain-specific duplicates pages and know what you are doing, you can turn this off.",
        kind: "bool",
        min: None,
        max: None,
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "hide_duplicates_needs_work_message_when_reasonably_caught_up",
        label: "Hide the \"x% done\" notification on preparation tab when >99% searched:",
        help: "By default, the duplicates filter page will not highlight that there is potential duplicates search work to do if you are 99% done. This saves you being notified by the normal background burble of regular file imports. If you want to know whenever any work is pending, uncheck this.",
        kind: "bool",
        min: None,
        max: None,
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_weights_help",
        label: "",
        help: "",
        kind: "readonly",
        min: None,
        max: None,
        noneable: false,
        none_phrase: "",
        readonly_value: "When processing potential duplicate pairs in the duplicate filter, the client tries to present the 'best' file first. It judges the two files on a variety of potential differences, each with a score. The file with the greatest total score is presented first. Here you can tinker with these scores.\n\nI recommend you leave all these as positive numbers, but if you wish, you can set a negative number to reduce the score.",
    },
    DuplicateField {
        key: "duplicate_comparison_score_higher_jpeg_quality",
        label: "Score for jpeg with non-trivially higher jpeg quality:",
        help: "",
        kind: "int",
        min: Some(-100),
        max: Some(100),
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_comparison_score_much_higher_jpeg_quality",
        label: "Score for jpeg with significantly higher jpeg quality:",
        help: "",
        kind: "int",
        min: Some(-100),
        max: Some(100),
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_comparison_score_higher_filesize",
        label: "Score for file with non-trivially higher filesize:",
        help: "",
        kind: "int",
        min: Some(-100),
        max: Some(100),
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_comparison_score_much_higher_filesize",
        label: "Score for file with significantly higher filesize:",
        help: "",
        kind: "int",
        min: Some(-100),
        max: Some(100),
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_comparison_score_higher_resolution",
        label: "Score for file with higher resolution (as num pixels):",
        help: "",
        kind: "int",
        min: Some(-100),
        max: Some(100),
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_comparison_score_much_higher_resolution",
        label: "Score for file with significantly higher resolution (as num pixels):",
        help: "",
        kind: "int",
        min: Some(-100),
        max: Some(100),
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_comparison_score_more_tags",
        label: "Score for file with more tags:",
        help: "",
        kind: "int",
        min: Some(-100),
        max: Some(100),
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_comparison_score_older",
        label: "Score for file with non-trivially earlier import time:",
        help: "",
        kind: "int",
        min: Some(-100),
        max: Some(100),
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_comparison_score_nicer_ratio",
        label: "Score for file with 'nicer' resolution ratio:",
        help: "For instance, 16:9 vs 640:357.",
        kind: "int",
        min: Some(-100),
        max: Some(100),
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_comparison_score_has_audio",
        label: "Score for file with audio:",
        help: "",
        kind: "int",
        min: Some(-100),
        max: Some(100),
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_filter_max_batch_size",
        label: "Max size of duplicate filter pair batches (in mixed mode):",
        help: "In group mode you always see the whole group, which in some cases can be 1,000+ items.",
        kind: "int",
        min: Some(5),
        max: Some(1024),
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_filter_auto_commit_batch_size",
        label: "Auto-commit completed batches of this size or smaller:",
        help: "When you are dealing with numerous 1/1 size batches/groups, it can get annoying to click through the confirm every time. This will auto-confirm any batch with this many decisions of fewer, assuming no manual skips.",
        kind: "noneable_int",
        min: Some(1),
        max: Some(50),
        noneable: true,
        none_phrase: "no, always confirm",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_colours_help",
        label: "",
        help: "",
        kind: "readonly",
        min: None,
        max: None,
        noneable: false,
        none_phrase: "",
        readonly_value: "The duplicate filter can darken/lighten your normal background colour. This highlights the transitions between A and B and, if your background colour is normally pure white or black, can differentiate transparency vs white/black opaque image background.",
    },
    DuplicateField {
        key: "duplicate_background_switch_intensity_a",
        label: "background light/dark switch intensity for A:",
        help: "This changes the background colour when you are looking at A. If you have a pure white/black background and do not have transparent images to show with checkerboard, it helps to highlight transparency vs opaque white/black image background.",
        kind: "noneable_int",
        min: Some(1),
        max: Some(9),
        noneable: true,
        none_phrase: "do not change",
        readonly_value: "",
    },
    DuplicateField {
        key: "duplicate_background_switch_intensity_b",
        label: "background light/dark switch intensity for B:",
        help: "This changes the background colour when you are looking at B. Making it different to the A value helps to highlight switches between the two.",
        kind: "noneable_int",
        min: Some(1),
        max: Some(9),
        noneable: true,
        none_phrase: "do not change",
        readonly_value: "",
    },
    DuplicateField {
        key: "draw_transparency_checkerboard_media_canvas_duplicates",
        label: "draw image transparency as checkerboard in the duplicate filter:",
        help: "Same as the setting in _media playback_, but only for the duplicate filter.",
        kind: "bool",
        min: None,
        max: None,
        noneable: false,
        none_phrase: "",
        readonly_value: "",
    },
];

pub fn duplicates_field(key: &str) -> Option<&'static DuplicateField> {
    DUPLICATE_FIELDS.iter().find(|field| field.key == key)
}

pub fn duplicates_labels() -> Vec<&'static str> {
    DUPLICATE_FIELDS.iter().map(|field| field.label).collect()
}

pub fn duplicates_help(key: &str) -> &'static str {
    duplicates_field(key).map(|field| field.help).unwrap_or("")
}

pub fn duplicates_range(key: &str) -> Option<(i64, i64)> {
    let field = duplicates_field(key)?;
    match (field.min, field.max) {
        (Some(min), Some(max)) => Some((min, max)),
        _ => None,
    }
}

pub fn duplicates_none_phrase(key: &str) -> &'static str {
    duplicates_field(key).map(|field| field.none_phrase).unwrap_or("")
}

pub fn duplicates_clamp(key: &str, value: i64) -> i64 {
    if let Some((min, max)) = duplicates_range(key) {
        value.clamp(min, max)
    } else {
        value
    }
}

pub fn duplicates_clamp_noneable(key: &str, value: Option<i64>) -> Option<i64> {
    value.map(|value| duplicates_clamp(key, value))
}

pub fn general_labels() -> Vec<&'static str> {
    GENERAL_FIELDS.iter().map(|field| field.label).collect()
}

pub fn general_help(key: &str) -> &'static str {
    general_field(key).map(|field| field.help).unwrap_or("")
}

pub fn panel_order() -> &'static [&'static str] {
    &[
        "general", "notifications", "statistics", "files", "exporting", "tags",
        "thumbnails", "media", "maintenance", "downloads", "duplicates", "network", "api",
    ]
}

pub fn panel_title(name: &str) -> &'static str {
    match name {
        "general" => "general",
        "notifications" => "popup notifications",
        "statistics" => "file viewing statistics",
        "files" => "files and import",
        "exporting" => "exporting",
        "tags" => "tag presentation",
        "thumbnails" => "thumbnails",
        "media" => "media viewer",
        "maintenance" => "file maintenance",
        "downloads" => "downloaders",
        "duplicates" => "duplicates",
        "network" => "network",
        "api" => "client api",
        _ => "",
    }
}

pub fn format_field_value(kind: &str, value: &Value) -> String {
    match kind {
        "bool" => {
            if value.as_bool().unwrap_or(false) {
                "yes".to_string()
            } else {
                "no".to_string()
            }
        }
        "int" => value.as_i64().unwrap_or(0).to_string(),
        "string" => value.as_str().unwrap_or("").to_string(),
        "readonly" => value.as_str().unwrap_or("").to_string(),
        _ => String::new(),
    }
}

pub fn options_json(state: &AppState) -> String {
    let controller = &state.controller;
    let options = controller.options();

    let mut tabs = Vec::new();

    let mut general_fields = Vec::new();
    for field in GENERAL_FIELDS {
        let value = match field.kind {
            "bool" => json!(options.get_boolean(field.key)),
            "int" => json!(options.get_integer(field.key)),
            "string" => json!(options.get_string(field.key)),
            _ => json!(null),
        };
        general_fields.push(json!({
            "key": field.key,
            "label": field.label,
            "type": field.kind,
            "value": value,
            "help": field.help,
        }));
    }
    tabs.push(tab("general", "general", general_fields));

    // search bar placement is driven by the general option of the same name
    let search_bar_top = options.get_boolean("options_search_bar_top_of_window");

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

    let mut duplicate_fields = Vec::new();
    for field in DUPLICATE_FIELDS {
        let value = match field.kind {
            "bool" => json!(options.get_boolean(field.key)),
            "int" => json!(options.get_integer(field.key)),
            "noneable_int" => json!(options.get_noneable_integer(field.key)),
            "readonly" => json!(field.readonly_value),
            _ => json!(null),
        };
        duplicate_fields.push(json!({
            "key": field.key,
            "label": field.label,
            "type": field.kind,
            "value": value,
            "help": field.help,
            "min": field.min,
            "max": field.max,
            "noneable": field.noneable,
            "none_phrase": field.none_phrase,
        }));
    }
    tabs.push(tab("duplicates", "duplicates", duplicate_fields));

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
        "search_bar_top": search_bar_top,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn panel_order_starts_with_general() {
        let order = panel_order();
        assert_eq!(order[0], "general");
        assert!(order.contains(&"api"));
        assert_eq!(order.len(), 13);
    }

    #[test]
    fn panel_title_lookup() {
        assert_eq!(panel_title("general"), "general");
        assert_eq!(panel_title("media"), "media viewer");
        assert_eq!(panel_title("nope"), "");
    }

    #[test]
    fn general_field_label_lookup() {
        let field = general_field("advanced_mode").expect("field exists");
        assert_eq!(field.label, "show advanced controls");
        assert_eq!(field.kind, "bool");
        assert!(general_field("does_not_exist").is_none());
    }

    #[test]
    fn general_labels_include_expected() {
        let labels = general_labels();
        assert!(labels.contains(&"confirm closing pages"));
        assert!(labels.contains(&"loop gifs"));
    }

    #[test]
    fn general_help_lookup() {
        assert!(general_help("always_loop_gifs").contains("GIF"));
        assert_eq!(general_help("missing"), "");
    }

    #[test]
    fn format_field_value_bool() {
        assert_eq!(format_field_value("bool", &json!(true)), "yes");
        assert_eq!(format_field_value("bool", &json!(false)), "no");
    }

    #[test]
    fn format_field_value_int() {
        assert_eq!(format_field_value("int", &json!(42)), "42");
    }

    #[test]
    fn format_field_value_string() {
        assert_eq!(format_field_value("string", &json!("abc")), "abc");
    }

    #[test]
    fn duplicates_field_label_lookup() {
        let field = duplicates_field("duplicate_filter_max_batch_size").expect("field exists");
        assert_eq!(field.label, "Max size of duplicate filter pair batches (in mixed mode):");
        assert_eq!(field.kind, "int");
        assert!(duplicates_field("does_not_exist").is_none());
    }

    #[test]
    fn duplicates_labels_include_expected() {
        let labels = duplicates_labels();
        assert!(labels.contains(&"Score for file with audio:"));
        assert!(labels.contains(&"draw image transparency as checkerboard in the duplicate filter:"));
    }

    #[test]
    fn duplicates_help_lookup() {
        assert!(duplicates_help("duplicate_comparison_score_nicer_ratio").contains("16:9"));
        assert_eq!(duplicates_help("missing"), "");
    }

    #[test]
    fn duplicates_range_lookup() {
        assert_eq!(duplicates_range("duplicate_comparison_score_higher_jpeg_quality"), Some((-100, 100)));
        assert_eq!(duplicates_range("duplicate_filter_max_batch_size"), Some((5, 1024)));
        assert_eq!(duplicates_range("open_files_to_duplicate_filter_uses_all_my_files"), None);
    }

    #[test]
    fn duplicates_clamp_int() {
        assert_eq!(duplicates_clamp("duplicate_comparison_score_higher_jpeg_quality", 150), 100);
        assert_eq!(duplicates_clamp("duplicate_comparison_score_higher_jpeg_quality", -150), -100);
        assert_eq!(duplicates_clamp("duplicate_comparison_score_higher_jpeg_quality", 42), 42);
        assert_eq!(duplicates_clamp("open_files_to_duplicate_filter_uses_all_my_files", 7), 7);
    }

    #[test]
    fn duplicates_clamp_noneable_test() {
        assert_eq!(duplicates_clamp_noneable("duplicate_background_switch_intensity_a", Some(0)), Some(1));
        assert_eq!(duplicates_clamp_noneable("duplicate_background_switch_intensity_a", Some(12)), Some(9));
        assert_eq!(duplicates_clamp_noneable("duplicate_background_switch_intensity_a", None), None);
    }

    #[test]
    fn duplicates_none_phrase_lookup() {
        assert_eq!(duplicates_none_phrase("duplicate_filter_auto_commit_batch_size"), "no, always confirm");
        assert_eq!(duplicates_none_phrase("duplicate_background_switch_intensity_a"), "do not change");
    }
}
