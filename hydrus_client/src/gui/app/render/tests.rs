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
