//! Default values from `hydrus/client/ClientDefaults.py`.

use serde_json::{Map, Value, json};

use crate::client_constants;

mod colours;
mod network;

pub use colours::{darkmode_colours, default_colours};
pub use network::{
    CustomHeader, DEFAULT_USER_AGENT, apply_default_bandwidth_rules, default_custom_headers,
};

/// Legacy YAML-backed options (the `options` table).
pub fn legacy_options() -> Map<String, Value> {
    let mut options = Map::new();

    options.insert("export_path".to_string(), Value::Null);
    options.insert("hpos".to_string(), json!(400));
    options.insert("vpos".to_string(), json!(-240));
    options.insert("thumbnail_dimensions".to_string(), json!([150, 125]));
    options.insert("password".to_string(), Value::Null);
    options.insert(
        "default_gui_session".to_string(),
        json!(client_constants::LAST_SESSION_SESSION_NAME),
    );
    options.insert("idle_period".to_string(), json!(60 * 30));
    options.insert("idle_mouse_period".to_string(), json!(60 * 10));
    options.insert("idle_normal".to_string(), json!(true));
    options.insert(
        "idle_shutdown".to_string(),
        json!(client_constants::IDLE_ON_SHUTDOWN_ASK_FIRST),
    );
    options.insert("idle_shutdown_max_minutes".to_string(), json!(5));
    options.insert("trash_max_age".to_string(), json!(72));
    options.insert("trash_max_size".to_string(), json!(2048));
    options.insert("remove_trashed_files".to_string(), json!(false));
    options.insert("remove_filtered_files".to_string(), json!(false));
    options.insert("confirm_trash".to_string(), json!(true));
    options.insert("confirm_archive".to_string(), json!(true));
    options.insert("gallery_file_limit".to_string(), json!(2000));
    options.insert("delete_to_recycle_bin".to_string(), json!(true));
    options.insert("animation_start_position".to_string(), json!(0.0));
    options.insert("hide_preview".to_string(), json!(false));
    options.insert("regex_favourites".to_string(), json!([]));
    options.insert(
        "namespace_colours".to_string(),
        Value::Object(default_colours()),
    );
    options.insert("proxy".to_string(), Value::Null);
    options.insert("confirm_client_exit".to_string(), json!(false));

    options
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckerOptions {
    pub intended_files_per_check: i64,
    pub never_faster_than: i64,
    pub never_slower_than: i64,
    pub death_file_velocity: (i64, i64),
}

/// `get_default_checker_options( name )`
pub fn get_default_checker_options(name: &str) -> Option<CheckerOptions> {
    const DAY: i64 = 86_400;

    let options = match name {
        "thread" => CheckerOptions {
            intended_files_per_check: 4,
            never_faster_than: 300,
            never_slower_than: DAY,
            death_file_velocity: (1, 3 * DAY),
        },
        "slow thread" => CheckerOptions {
            intended_files_per_check: 1,
            never_faster_than: 4 * 3_600,
            never_slower_than: 7 * DAY,
            death_file_velocity: (1, 30 * DAY),
        },
        "artist subscription" => CheckerOptions {
            intended_files_per_check: 4,
            never_faster_than: DAY,
            never_slower_than: 90 * DAY,
            death_file_velocity: (1, 180 * DAY),
        },
        "fast tag subscription" => CheckerOptions {
            intended_files_per_check: 10,
            never_faster_than: 43_200,
            never_slower_than: 30 * DAY,
            death_file_velocity: (1, 90 * DAY),
        },
        "slow tag subscription" => CheckerOptions {
            intended_files_per_check: 1,
            never_faster_than: 7 * DAY,
            never_slower_than: 180 * DAY,
            death_file_velocity: (1, 365 * DAY),
        },
        _ => return None,
    };

    Some(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_options_match_python_defaults() {
        let options = legacy_options();

        assert_eq!(options["thumbnail_dimensions"], json!([150, 125]));
        assert_eq!(options["delete_to_recycle_bin"], json!(true));
        assert_eq!(options["idle_shutdown"], json!(2));
    }

    #[test]
    fn checker_option_names_match_python() {
        for name in [
            "thread",
            "slow thread",
            "artist subscription",
            "fast tag subscription",
            "slow tag subscription",
        ] {
            assert!(get_default_checker_options(name).is_some(), "{name}");
        }

        assert!(get_default_checker_options("unknown").is_none());
    }

    #[test]
    fn thread_checker_has_python_values() {
        let options = get_default_checker_options("thread").unwrap();

        assert_eq!(options.intended_files_per_check, 4);
        assert_eq!(options.never_faster_than, 300);
        assert_eq!(options.death_file_velocity, (1, 3 * 86_400));
    }
}
