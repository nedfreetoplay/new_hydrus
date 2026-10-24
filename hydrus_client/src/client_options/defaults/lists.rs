//! Integer lists, service keys and colour sets.

use serde_json::{Map, Value, json};

use crate::client_constants;
use crate::client_defaults::{darkmode_colours, default_colours};

pub(super) fn integer_list() -> Map<String, Value> {
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

pub(super) fn keys() -> Map<String, Value> {
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

pub(super) fn colours() -> Value {
    let mut sets = Map::new();
    sets.insert("default".to_string(), Value::Object(default_colours()));
    sets.insert("darkmode".to_string(), Value::Object(darkmode_colours()));

    Value::Object(sets)
}

pub(super) fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
