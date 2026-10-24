//! The default `ClientOptions` dictionary.
//!
//! The Python object defaults to several hundred keys. The port starts with the
//! keys used by the model, database and GUI shell; new keys are added next to
//! the feature that needs them.

use serde_json::{Map, Value, json};

use crate::client_constants;
use crate::client_defaults::{darkmode_colours, default_colours};

pub fn default_options_dictionary() -> Map<String, Value> {
    let mut dictionary = Map::new();

    dictionary.insert("booleans".to_string(), Value::Object(booleans()));
    dictionary.insert("integers".to_string(), Value::Object(integers()));
    dictionary.insert("floats".to_string(), Value::Object(floats()));
    dictionary.insert("strings".to_string(), Value::Object(strings()));
    dictionary.insert(
        "noneable_integers".to_string(),
        Value::Object(noneable_integers()),
    );
    dictionary.insert(
        "noneable_strings".to_string(),
        Value::Object(noneable_strings()),
    );
    dictionary.insert("string_list".to_string(), Value::Object(string_list()));
    dictionary.insert("integer_list".to_string(), Value::Object(integer_list()));
    dictionary.insert("keys".to_string(), Value::Object(keys()));
    dictionary.insert("key_list".to_string(), Value::Object(Map::new()));
    dictionary.insert("colours".to_string(), colours());

    dictionary
}

fn booleans() -> Map<String, Value> {
    let entries: &[(&str, bool)] = &[
        ("advanced_mode", false),
        ("use_nice_resolution_strings", true),
        ("show_namespaces", true),
        ("show_number_namespaces", true),
        ("show_subtag_number_namespaces", true),
        ("replace_tag_underscores_with_spaces", false),
        ("always_loop_gifs", true),
        ("file_viewing_statistics_active", true),
        ("use_qt_file_dialogs", false),
        ("confirm_all_page_closes", false),
        ("activate_window_on_tag_search_page_activation", false),
        ("set_search_focus_on_page_change", false),
        ("hide_uninteresting_modified_time", true),
        ("draw_transparency_checkerboard_media_canvas", false),
        ("use_system_ffmpeg", false),
        ("do_icc_profile_normalisation", true),
        ("show_extended_single_file_info_in_status_bar", true),
        ("only_save_last_session_during_idle", false),
        ("saving_sash_positions_on_exit", true),
        ("default_search_synchronised", true),
        ("remember_options_window_panel", true),
        ("options_search_bar_top_of_window", true),
        ("allow_blurhash_fallback", true),
        ("fade_thumbnails", true),
        ("file_system_waits_on_wakeup", false),
        ("do_sleep_check", true),
        ("confirm_multiple_local_file_services_move", true),
        ("confirm_multiple_local_file_services_copy", true),
        ("page_drop_chase_normally", true),
        ("page_drag_change_tab_normally", true),
        ("verify_regular_https", true),
        ("process_subs_in_random_order", true),
        ("maintain_trash_in_normal_time", true),
        ("deferred_file_deletes_in_normal_time", true),
        ("prefix_hash_when_copying", false),
        ("always_show_iso_time", false),
        ("expand_parents_on_storage_taglists", true),
        ("show_parent_decorators_on_storage_taglists", true),
        ("show_sibling_decorators_on_storage_taglists", true),
        ("global_audio_mute", false),
        ("media_viewer_audio_mute", false),
        ("use_advanced_file_deletion_dialog", false),
    ];

    entries
        .iter()
        .map(|(name, value)| (name.to_string(), json!(value)))
        .collect()
}

fn integers() -> Map<String, Value> {
    let entries: &[(&str, i64)] = &[
        ("video_buffer_size", 96 * 1024 * 1024),
        ("related_tags_concurrence_threshold_percent", 6),
        ("suggested_tags_width", 300),
        ("similar_files_duplicate_pairs_search_distance", 0),
        (
            "default_new_page_goes",
            client_constants::NEW_PAGE_GOES_FAR_RIGHT as i64,
        ),
        (
            "close_page_focus_goes",
            client_constants::CLOSED_PAGE_FOCUS_GOES_RIGHT as i64,
        ),
        ("num_recent_petition_reasons", 5),
        ("max_page_name_chars", 20),
        (
            "page_file_count_display",
            client_constants::PAGE_FILE_COUNT_DISPLAY_ALL_BUT_ONLY_IF_GREATER_THAN_ZERO as i64,
        ),
        ("network_timeout", 10),
        ("connection_error_wait_time", 15),
        ("serverside_bandwidth_wait_time", 60),
        ("thumbnail_visibility_scroll_percent", 75),
        ("ideal_tile_dimension", 768),
        ("wake_delay_period", 15),
        ("last_session_save_period_minutes", 5),
        ("shutdown_work_period", 86_400),
        ("max_network_jobs", 15),
        ("max_network_jobs_per_domain", 3),
        ("max_connection_attempts_allowed", 5),
        ("max_request_attempts_allowed_get", 5),
        ("popup_message_character_width", 56),
        ("duplicate_filter_max_batch_size", 100),
        ("video_thumbnail_percentage_in", 35),
        ("global_audio_volume", 70),
        ("media_viewer_audio_volume", 70),
        ("preview_audio_volume", 70),
        ("thumbnail_cache_size", 1024 * 1024 * 32),
        ("image_cache_size", 1024 * 1024 * 1024),
        ("image_tile_cache_size", 1024 * 1024 * 256),
        ("thumbnail_cache_timeout", 86_400),
        ("image_cache_timeout", 600),
        ("image_tile_cache_timeout", 300),
        ("thumbnail_border", 1),
        ("thumbnail_margin", 2),
        ("thumbnail_dpr_percent", 100),
        ("number_of_gui_session_backups", 10),
        ("animated_scanbar_height", 20),
        ("animated_scanbar_nub_width", 10),
        ("ac_read_list_height_num_chars", 22),
        ("ac_write_list_height_num_chars", 11),
        ("human_bytes_sig_figs", 3),
        ("thread_slots_misc", 10),
        ("thread_slots_gallery_files", 15),
        ("thread_slots_gallery_search", 5),
        ("thread_slots_watcher_files", 15),
        ("thread_slots_watcher_check", 5),
        ("ffmpeg_subprocess_timeout", 15),
        ("page_nav_history_max_entries", 100),
        ("treeview_indentation", 20),
        ("treeview_row_height", 20),
        (
            "media_viewer_tags_scrolling_behaviour",
            client_constants::MEDIA_VIEWER_TAGS_SCROLLING_BEHAVIOUR_ONLY_PROPAGATE_AFTER_DELAY as i64,
        ),
    ];

    entries
        .iter()
        .map(|(name, value)| (name.to_string(), json!(value)))
        .collect()
}

fn floats() -> Map<String, Value> {
    Map::new()
}

fn strings() -> Map<String, Value> {
    let entries: &[(&str, &str)] = &[
        ("app_display_name", "hydrus client"),
        ("namespace_connector", ":"),
        ("sibling_connector", " \u{2192} "),
        ("or_connector", " OR "),
        ("export_phrase", "{hash}"),
        ("current_colourset", "default"),
        ("favourite_simple_downloader_formula", "all files linked by images in page"),
        ("thumbnail_scroll_rate", "1.0"),
        ("pause_character", "\u{23f8}"),
        ("stop_character", "\u{23f9}"),
        ("has_audio_label", "\u{1f50a}"),
        ("has_duration_label", " \u{23f5} "),
        ("default_suggested_tags_notebook_page", "related"),
        ("last_incremental_tagging_namespace", "page"),
        ("last_incremental_tagging_prefix", ""),
        ("last_incremental_tagging_suffix", ""),
        ("last_options_window_panel", "gui"),
        ("page_of_pages_decorator", " \u{2193}"),
    ];

    entries
        .iter()
        .map(|(name, value)| (name.to_string(), json!(value)))
        .collect()
}

fn noneable_integers() -> Map<String, Value> {
    let entries: &[(&str, Option<i64>)] = &[
        ("forced_search_limit", None),
        ("num_recent_tags", Some(20)),
        ("num_recent_tags_in_quick_dialog", Some(10)),
        ("duplicate_background_switch_intensity_a", Some(0)),
        ("duplicate_background_switch_intensity_b", Some(3)),
        ("duplicate_filter_auto_commit_batch_size", Some(1)),
        ("media_viewer_cursor_autohide_time_ms", Some(700)),
        ("animated_scanbar_hide_height", Some(5)),
        ("last_backup_time", None),
        ("export_path_character_limit", None),
        ("export_dirname_character_limit", None),
    ];

    entries
        .iter()
        .map(|(name, value)| (name.to_string(), json!(value)))
        .collect()
}

fn noneable_strings() -> Map<String, Value> {
    let entries: &[(&str, Option<&str>)] = &[
        ("favourite_file_lookup_script", None),
        ("suggested_tags_layout", Some("notebook")),
        ("backup_path", None),
        ("last_png_export_dir", None),
        ("media_background_bmp_path", None),
        ("http_proxy", None),
        ("https_proxy", None),
        ("no_proxy", Some("127.0.0.1")),
        ("qt_style_name", None),
        ("qt_stylesheet_name", None),
    ];

    entries
        .iter()
        .map(|(name, value)| (name.to_string(), json!(value)))
        .collect()
}

fn string_list() -> Map<String, Value> {
    let mut map = Map::new();

    map.insert(
        "default_media_viewer_custom_shortcuts".to_string(),
        json!([]),
    );
    map.insert("favourite_tags".to_string(), json!([]));
    map.insert(
        "advanced_file_deletion_reasons".to_string(),
        json!([
            "I do not like it.",
            "It is bad quality.",
            "It is not appropriate for this client.",
            "Temporary delete--I want to bring it back later."
        ]),
    );
    map.insert(
        "user_namespace_group_by_sort".to_string(),
        json!(["creator", "series", "character", "species", "", "meta"]),
    );

    map
}

fn integer_list() -> Map<String, Value> {
    let mut map = Map::new();

    map.insert(
        "file_viewing_stats_interesting_canvas_types".to_string(),
        json!([
            client_constants::CANVAS_MEDIA_VIEWER,
            client_constants::CANVAS_CLIENT_API
        ]),
    );
    map.insert(
        "command_palette_provider_order".to_string(),
        json!([
            client_constants::COMMAND_PALETTE_PROVIDER_CALCULATOR,
            client_constants::COMMAND_PALETTE_PROVIDER_MAIN_MENU,
            client_constants::COMMAND_PALETTE_PROVIDER_MEDIA_MENU,
            client_constants::COMMAND_PALETTE_PROVIDER_PAGES_HISTORY,
            client_constants::COMMAND_PALETTE_PROVIDER_PAGES,
            client_constants::COMMAND_PALETTE_PROVIDER_FAVOURITE_SEARCH
        ]),
    );
    map.insert(
        "tab_tree_splitter_sizes_left".to_string(),
        json!([200, 1000]),
    );
    map.insert(
        "tab_tree_splitter_sizes_right".to_string(),
        json!([1000, 200]),
    );
    map.insert(
        "treeview_expanding_panel_splitter_size".to_string(),
        json!([700, 150]),
    );

    map
}

fn keys() -> Map<String, Value> {
    let mut map = Map::new();

    map.insert(
        "default_tag_service_tab".to_string(),
        json!(hex_encode(b"local tags")),
    );
    map.insert(
        "default_tag_service_search_page".to_string(),
        json!(hex_encode(b"all known tags")),
    );
    map.insert(
        "options_ratings_panel_template_service_key".to_string(),
        json!(hex_encode(b"preview ratings")),
    );

    map
}

fn colours() -> Value {
    let mut sets = Map::new();
    sets.insert(
        "default".to_string(),
        Value::Object(default_colours()),
    );
    sets.insert(
        "darkmode".to_string(),
        Value::Object(darkmode_colours()),
    );

    Value::Object(sets)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
