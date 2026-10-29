use std::path::{Path, PathBuf};

use serde_json::Value;

use super::*;
use crate::gui::app::model::AppState;

fn temp_dir(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_file_locations_{label}_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

fn state(directory: &Path) -> AppState {
    let _ = crate::client_options::register_serialisable_type();

    let controller = crate::controller::Controller::new(directory).unwrap();
    controller.init_model().unwrap();

    AppState::new(std::sync::Arc::new(controller))
}

fn dialog(state: &AppState) -> Value {
    serde_json::from_str(&dialog_json(state)).unwrap()
}

#[test]
fn opening_lists_every_subfolder_with_its_status() {
    let directory = temp_dir("open");
    let mut app = state(&directory);

    let commands = open(&mut app);
    assert!(matches!(
        commands.first(),
        Some(crate::gui::app::UiCommand::RepairFileLocationsDialog(_))
    ));

    let value = dialog(&app);

    assert_eq!(value["rows"].as_array().unwrap().len(), 2);
    assert_eq!(value["missing_files"], 0);
    assert_eq!(value["missing_thumbs"], 0);
    assert!(
        value["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["status"] == "ok!")
    );

    drop(app);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn probing_a_base_folder_fills_in_a_missing_thumbnail_path() {
    let directory = temp_dir("probe");
    let mut app = state(&directory);
    let thumbnails = directory.join("thumbs");
    std::fs::remove_dir_all(&thumbnails).unwrap();

    let _ = open(&mut app);
    assert_eq!(dialog(&app)["missing_thumbs"], 1);

    let root = directory.join("moved");
    std::fs::create_dir_all(root.join("thumbnails")).unwrap();
    let payload = serde_json::json!({"path": root.display().to_string()}).to_string();
    let _ = probe(&mut app, &payload);

    let value = dialog(&app);
    assert_eq!(value["missing_thumbs"], 0);
    assert!(
        value["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["prefix"] == "t" && row["exists"] == true)
    );

    let _ = apply(&mut app, r#"{"create_thumbnails": true}"#);
    assert!(thumbnails.exists() || root.join("thumbnails").exists());
    assert_eq!(
        app.controller
            .files_manager()
            .subfolders()
            .iter()
            .find(|subfolder| subfolder.prefix == "t")
            .unwrap()
            .base_location
            .path,
        root.join("thumbnails")
    );

    drop(app);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn set_locations_updates_the_dialog_row() {
    let directory = temp_dir("set");
    let mut app = state(&directory);
    let _ = open(&mut app);

    let new_base = directory.join("media");
    std::fs::create_dir_all(&new_base).unwrap();
    let payload = serde_json::json!({
        "updates": [{"prefix": "f", "base": new_base.display().to_string()}],
    })
    .to_string();
    let _ = set_locations(&mut app, &payload);

    let value = dialog(&app);
    let file_row = value["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["prefix"] == "f")
        .unwrap();

    assert_eq!(file_row["base"], new_base.display().to_string().as_str());
    assert_eq!(file_row["exists"], true);

    drop(app);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn apply_refuses_a_missing_file_prefix() {
    let directory = temp_dir("refuse");
    let mut app = state(&directory);
    let _ = open(&mut app);

    let missing = directory.join("not-there");
    let payload = serde_json::json!({
        "updates": [{"prefix": "f", "base": missing.display().to_string()}],
    })
    .to_string();
    let _ = set_locations(&mut app, &payload);
    let _ = apply(&mut app, r#"{"create_thumbnails": true}"#);

    let value = dialog(&app);
    assert!(
        value["error"]
            .as_str()
            .unwrap()
            .contains("is still missing")
    );
    assert!(
        app.controller
            .files_manager()
            .subfolders()
            .iter()
            .find(|subfolder| subfolder.prefix == "f")
            .unwrap()
            .base_location
            .path
            != missing
    );

    drop(app);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn apply_writes_subfolders_and_refreshes_the_files_manager() {
    let directory = temp_dir("apply");
    let mut app = state(&directory);
    let _ = open(&mut app);

    let new_base = directory.join("new-media");
    std::fs::create_dir_all(&new_base).unwrap();
    let payload = serde_json::json!({
        "updates": [{"prefix": "f", "base": new_base.display().to_string()}],
    })
    .to_string();
    let _ = set_locations(&mut app, &payload);

    let commands = apply(&mut app, r#"{"create_thumbnails": true}"#);

    assert!(matches!(
        commands.first(),
        Some(crate::gui::app::UiCommand::CloseRepairFileLocations)
    ));
    assert!(new_base.exists());
    assert_eq!(
        app.controller
            .files_manager()
            .subfolders()
            .iter()
            .find(|subfolder| subfolder.prefix == "f")
            .unwrap()
            .base_location
            .path,
        new_base
    );

    let stored =
        crate::db::files_physical_storage::get_current_locations(app.controller.db().db()).unwrap();
    assert!(stored.iter().any(|location| location.path == new_base));

    drop(app);
    let _ = std::fs::remove_dir_all(directory);
}
