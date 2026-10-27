use super::*;
use crate::controller::Controller;
use crate::db::{files_viewing_stats, master};
use crate::gui::app::model::{ImportQueueState, QueryPage};

#[test]
fn query_state_carries_count_loading_and_thumbnail_options() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_gui_query_state_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();

    let mut page = QueryPage::new("p1", "files", "my files");
    page.hashes = vec![vec![1], vec![2], vec![3]];
    page.loading = true;

    let import = ImportQueueState {
        running: true,
        done: 2,
        total: 5,
        imported: 1,
        failed: 0,
        message: "importing".to_string(),
    };

    let state = query_state(&controller, &page, Some(&import));

    assert_eq!(state["count"], 3);
    assert_eq!(state["loading"], true);
    assert_eq!(state["importing"]["done"], 2);
    assert_eq!(state["importing"]["total"], 5);
    assert_eq!(state["thumbnails"]["scale_percent"], 100);
    assert_eq!(state["thumbnails"]["masonry"], false);
    assert_eq!(state["thumbnails"]["horizontal"], false);

    let mut options = controller.options();
    options.set_integer("thumbnail_scale_percent", 175);
    options.set_boolean("thumbnail_layout_masonry", true);
    options.set_boolean("thumbnail_scroll_horizontal", true);
    controller.replace_options(options);

    page.loading = false;
    let state = query_state(&controller, &page, None);

    assert_eq!(state["loading"], false);
    assert_eq!(state["importing"], Value::Null);
    assert_eq!(state["thumbnails"]["scale_percent"], 175);
    assert_eq!(state["thumbnails"]["masonry"], true);
    assert_eq!(state["thumbnails"]["horizontal"], true);

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn maintenance_json_lists_queued_jobs() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_gui_maintenance_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = std::sync::Arc::new(Controller::new(&directory).unwrap());
    controller.init_model().unwrap();

    controller.maintenance().schedule_job_now(
        vec![0x11; 32],
        crate::files::maintenance::REGENERATE_FILE_DATA_JOB_FILE_MODIFIED_TIMESTAMP,
    );

    let state = crate::gui::app::model::AppState::new(controller);
    let text = maintenance_json(&state);

    assert!(text.contains("1 file maintenance job(s) queued"));
    assert!(text.contains("regenerate file modified time"));

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn about_json_reports_versions_and_paths() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_gui_about_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();

    let state = crate::gui::app::model::AppState::new(std::sync::Arc::new(controller));
    let json: Value = serde_json::from_str(&about_json(&state)).unwrap();

    assert_eq!(json["software_version"], 689);
    assert_eq!(json["network_version"], 20);
    assert!(json["db_version"].as_i64().unwrap() > 0);
    assert!(json["sqlite"].as_str().unwrap().starts_with('3'));
    assert_eq!(json["db_dir"], directory.display().to_string());
    assert_eq!(json["api"], "stopped");
    assert!(json["services"].as_i64().unwrap() > 0);
    assert!(json["site"].as_str().unwrap().starts_with("https://"));

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn viewing_stats_json_lists_media_and_preview() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_gui_viewing_stats_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();
    let db = controller.db().db();
    let hash = vec![0x71; 32];
    let hash_id = master::get_hash_id(db, &hash).unwrap();

    files_viewing_stats::add_viewing_stats(db, hash_id, 0, 4, 2000, Some(1_700_000_000_000))
        .unwrap();

    let json: Value = serde_json::from_str(&viewing_stats_json(&controller, &hash)).unwrap();
    let rows = json["rows"].as_array().unwrap();

    assert_eq!(
        json["hash"],
        "7171717171717171717171717171717171717171717171717171717171717171"
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["canvas"], "media");
    assert_eq!(rows[0]["views"], 4);
    assert_eq!(rows[0]["viewtime"], 2);
    assert_eq!(rows[1]["canvas"], "preview");
    assert_eq!(rows[1]["views"], 0);

    let _ = std::fs::remove_dir_all(directory);
}
