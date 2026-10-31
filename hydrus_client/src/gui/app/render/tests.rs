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
fn about_reports_versions_and_paths() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_gui_about_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();

    let state = crate::gui::app::model::AppState::new(std::sync::Arc::new(controller));
    let info = about(&state);

    assert_eq!(info.software_version, 689);
    assert_eq!(info.network_version, 20);
    assert!(info.db_version > 0);
    assert!(info.sqlite.starts_with('3'));
    assert_eq!(info.db_dir, directory.display().to_string());
    assert_eq!(info.api, "stopped");
    assert!(info.services > 0);
    assert!(info.site.starts_with("https://"));

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn viewing_stats_lists_media_and_preview() {
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

    let stats = viewing_stats(&controller, &hash);

    assert_eq!(
        stats.hash,
        "7171717171717171717171717171717171717171717171717171717171717171"
    );
    assert_eq!(stats.rows.len(), 2);
    assert_eq!(stats.rows[0].canvas, "media");
    assert_eq!(stats.rows[0].views, 4);
    assert_eq!(stats.rows[0].viewtime_secs, 2);
    assert_eq!(stats.rows[1].canvas, "preview");
    assert_eq!(stats.rows[1].views, 0);

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn status_text_helpers_cover_query_downloader_and_busy() {
    let mut page = QueryPage::new("p1", "files", "my files");
    page.hashes = vec![vec![1], vec![2], vec![3]];
    page.status = "searching".to_string();

    page.focused = Some(vec![2]);
    page.selected = Vec::new();
    assert_eq!(query_page_media_text(&page), "2/3 - searching");

    page.focused = None;
    page.selected = vec![vec![1], vec![2]];
    assert_eq!(query_page_media_text(&page), "2 selected - searching");

    let mut watcher = crate::gui::app::model::DownloaderPage::new(
        "d1",
        crate::gui::app::model::DownloaderKind::Watcher,
    );
    assert_eq!(
        downloader_page_media_text(&watcher),
        "no highlighted watcher"
    );
    watcher.url = "https://example.com/feed".to_string();
    assert_eq!(
        downloader_page_media_text(&watcher),
        "no files for this watcher and its publishing settings"
    );

    let gallery = crate::gui::app::model::DownloaderPage::new(
        "d2",
        crate::gui::app::model::DownloaderKind::Gallery,
    );
    assert_eq!(
        downloader_page_media_text(&gallery),
        "no highlighted query"
    );

    let idle_import = ImportQueueState {
        running: false,
        done: 0,
        total: 0,
        imported: 0,
        failed: 0,
        message: String::new(),
    };
    assert_eq!(busy_text(&idle_import, None), "");
    assert_eq!(idle_text(""), "idle");
    assert_eq!(idle_text("1/2"), "");
    assert_eq!(bandwidth_text(false), "0B");
    assert_eq!(bandwidth_text(true), "network paused");

    let running_import = ImportQueueState {
        running: true,
        done: 1,
        total: 2,
        imported: 1,
        failed: 0,
        message: String::new(),
    };
    assert_eq!(busy_text(&running_import, None), "1/2");
    assert_eq!(
        busy_text(&idle_import, Some(("gallery", 3, 7))),
        "gallery: 3/7"
    );
}
