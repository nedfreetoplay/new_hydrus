use super::*;
use crate::controller::Controller;
use crate::db::{files_viewing_stats, master};

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
