use super::*;
use hydrus_core::hydrus_serialisable::SerialisableObject;

#[test]
fn defaults_round_trip_through_accessors() {
    let mut options = ClientOptions::new();

    assert!(!options.get_boolean("advanced_mode"));
    assert!(options.get_boolean("use_nice_resolution_strings"));
    assert_eq!(options.get_integer("human_bytes_sig_figs"), 3);
    assert_eq!(options.get_string("namespace_connector"), ":");
    assert_eq!(options.get_noneable_integer("num_recent_tags"), Some(20));
    assert_eq!(
        options.get_noneable_string("no_proxy").as_deref(),
        Some("127.0.0.1")
    );
    assert_eq!(
        options.get_integer_list("tab_tree_splitter_sizes_left"),
        vec![200, 1000]
    );
    assert_eq!(
        options.get_colour(crate::client_constants::COLOUR_THUMB_BACKGROUND),
        Some((255, 255, 255))
    );

    options.set_boolean("advanced_mode", true);
    options.set_integer("human_bytes_sig_figs", 5);

    assert!(options.get_boolean("advanced_mode"));
    assert_eq!(options.get_integer("human_bytes_sig_figs"), 5);
}

#[test]
fn serialisable_tuple_matches_python_layout() {
    let options = ClientOptions::new();
    let value = options.get_serialisable_tuple();

    assert_eq!(value[0], json!(22));
    assert_eq!(value[1], json!(8));
    assert!(value[2].is_object());
}

#[test]
fn missing_defaults_are_filled_after_an_old_load() {
    let mut options = ClientOptions::new();
    let info = json!({
        "booleans": { "advanced_mode": true },
        "integers": { "human_bytes_sig_figs": 5 }
    });

    options.initialise_from_serialisable_info(&info).unwrap();
    options.merge_missing(&ClientOptions::new());

    assert!(options.get_boolean("advanced_mode"));
    assert_eq!(options.get_integer("human_bytes_sig_figs"), 5);
    assert_eq!(options.get_integer("api_port"), 45869);
    assert_eq!(options.get_string("api_access_key"), "");
}

#[test]
fn unknown_keys_survive_initialisation() {
    let mut options = ClientOptions::new();
    let info = json!({
        "booleans": { "advanced_mode": true, "custom_key": true },
        "custom_group": { "x": 1 }
    });

    options.initialise_from_serialisable_info(&info).unwrap();

    assert!(options.get_boolean("custom_key"));
    assert_eq!(options.get_dictionary()["custom_group"]["x"], json!(1));
}
