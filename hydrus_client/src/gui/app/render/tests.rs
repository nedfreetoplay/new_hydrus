use super::*;
use crate::controller::Controller;
use crate::db::{files_viewing_stats, master};
use crate::gui::app::model::{
    DownloaderKind, DownloaderPage, ImportQueueState, Notebook, PageNode, PlaceholderPage,
    QueryPage,
};
use crate::gui::qt::api::ffi::{PageKind, UiAction};

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
fn downloader_status_text_matches_the_old_cpp_formatting() {
    let now = 1_000_000;
    assert_eq!(relative_added_text(0, now, "now"), "now");
    assert_eq!(relative_added_text(now - 30, now, "now"), "just now");
    assert_eq!(relative_added_text(now - 120, now, ""), "2 minutes ago");
    assert_eq!(relative_added_text(now - 7_200, now, ""), "2 hours ago");
    assert_eq!(relative_added_text(now - 172_800, now, ""), "2 days ago");

    assert_eq!(
        downloader_status_lines(true, false, 0, 0, 0, 0),
        ("waiting for new watchers".to_string(), String::new())
    );
    assert_eq!(
        downloader_status_lines(true, true, 3, 5, 2, 1),
        (
            "1 watchers - 3/5".to_string(),
            "2 successful, 1 failed".to_string()
        )
    );
    assert_eq!(
        downloader_status_lines(false, false, 0, 0, 4, 2),
        ("waiting for new queries".to_string(), String::new())
    );
    assert_eq!(
        downloader_status_lines(false, true, 0, 0, 0, 3),
        (
            "1 query - 0/0".to_string(),
            "0 successful, 3 failed".to_string()
        )
    );

    assert_eq!(downloader_imports_status(0, 0, 0), "");
    assert_eq!(downloader_imports_status(4, 0, 2), "4 successful");
    assert_eq!(downloader_imports_status(4, 2, 6), "4 successful, 2 failed");
    assert_eq!(downloader_files_status(0, 0), "");
    assert_eq!(downloader_files_status(3, 5), "3/5");
    assert_eq!(downloader_search_status(0, 0, 0), "");
    assert_eq!(downloader_search_status(2, 1, 5), "2 successful, 2 pending");
    assert_eq!(progress_percent(0, 0), 0);
    assert_eq!(progress_percent(3, 5), 60);
}

#[test]
fn duplicates_status_line_carries_position_only_with_pairs() {
    assert_eq!(duplicates_status_line("prepared", 0, 0), "prepared");
    assert_eq!(duplicates_status_line("prepared", 5, 0), "prepared  -  1/5");
    assert_eq!(duplicates_status_line("prepared", 5, 4), "prepared  -  5/5");
}

#[test]
fn subscription_page_row_formats_times_and_status() {
    let mut entry = crate::gui::app::model::SubscriptionEntry::new("sub", "https://example.com");
    entry.period = 300;
    entry.status = "ready".to_string();
    entry.paused = true;

    let row = super::subscription_page_row(&entry);

    assert_eq!(row.period_text, "300 s");
    assert_eq!(row.last_check_text, "never");
    assert!(row.next_text.starts_with("in "));
    assert_eq!(row.status_text, "paused - ready");
    assert_eq!(row.imported_text, "0");
    assert_eq!(row.failed_text, "0");
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
    assert_eq!(downloader_page_media_text(&gallery), "no highlighted query");

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
    assert_eq!(notification_summary(0), "0 message(s)");
    assert_eq!(notification_summary(1), "1 message(s)");
    assert_eq!(notification_summary(12), "12 message(s)");
    assert_eq!(
        busy_text(&idle_import, Some(("gallery", 3, 7))),
        "gallery: 3/7"
    );
}

fn testing_state(name: &str) -> (std::path::PathBuf, crate::gui::app::model::AppState) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_gui_1522a_{name}_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = std::sync::Arc::new(Controller::new(&directory).unwrap());
    controller.init_model().unwrap();
    let state = crate::gui::app::model::AppState::new(controller);
    (directory, state)
}

fn entry<'entries>(
    menus: &'entries MenuBar,
    action: UiAction,
) -> &'entries crate::gui::qt::api::ffi::MenuEntry {
    menus
        .entries
        .iter()
        .find(|entry| entry.action == action)
        .unwrap_or_else(|| panic!("menu action {action:?} is missing"))
}

#[test]
fn menus_flat_rows_keep_order_and_predicates() {
    let (directory, state) = testing_state("menus");

    let menus = menus(&state);

    let titles: Vec<&str> = menus
        .entries
        .iter()
        .filter(|entry| entry.depth == 0)
        .map(|entry| entry.title.as_str())
        .collect();
    assert_eq!(
        titles,
        vec![
            "&file",
            "&undo",
            "&pages",
            "&database",
            "&network",
            "&services",
            "&tags",
            "&help"
        ],
        "fresh database has no repository services, so &pending stays hidden"
    );

    let undo = entry(&menus, UiAction::Undo);
    assert!(!undo.enabled);

    for action in [
        UiAction::HistoryBack,
        UiAction::HistoryForward,
        UiAction::UndoSearch,
        UiAction::RedoSearch,
    ] {
        assert!(!entry(&menus, action).enabled, "{action:?} starts disabled");
    }

    let pause = entry(&menus, UiAction::PauseNetwork);
    assert!(pause.checkable);
    assert!(!pause.checked);

    let palette = entry(&menus, UiAction::CommandPalette);
    assert_eq!(palette.tooltip, "Ctrl+K");

    // Submenu contents sit one depth below their submenu row.
    let load = menus
        .entries
        .iter()
        .position(|entry| entry.label == "load" && entry.is_submenu)
        .expect("sessions load submenu is missing");
    assert_eq!(menus.entries[load].depth, 2);
    assert_eq!(menus.entries[load + 1].label, "no saved sessions");
    assert!(!menus.entries[load + 1].enabled);
    assert_eq!(menus.entries[load + 1].depth, 3);

    let recent = menus
        .entries
        .iter()
        .position(|entry| entry.label == "recent searches" && entry.is_submenu)
        .expect("recent searches submenu is missing");
    assert_eq!(menus.entries[recent + 1].label, "no recent searches");
    assert!(!menus.entries[recent + 1].enabled);

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn pages_flat_rows_keep_notebook_tree() {
    let (directory, mut state) = testing_state("pages");

    let mut nested = Notebook::new("nb1", "nested");
    nested.current = Some("p2".to_string());
    nested
        .children
        .push(PageNode::Query(QueryPage::new("p2", "files", "my files")));
    nested
        .children
        .push(PageNode::Downloader(DownloaderPage::new(
            "d1",
            DownloaderKind::Gallery,
        )));

    let mut p1 = QueryPage::new("p1", "files", "my files");
    p1.hashes = vec![vec![1], vec![2]];
    state.root.children.clear();
    state.root.children.push(PageNode::Query(p1));
    state.root.children.push(PageNode::Notebook(nested));
    state.root.current = Some("p1".to_string());
    state.active = Some("p2".to_string());

    let pages = pages(&state);

    assert_eq!(pages.current, "p1");
    assert_eq!(pages.rows.len(), 4);

    assert_eq!(pages.rows[0].id, "p1");
    assert_eq!(pages.rows[0].kind, PageKind::Query);
    assert_eq!(pages.rows[0].depth, 0);

    assert_eq!(pages.rows[1].id, "nb1");
    assert_eq!(pages.rows[1].kind, PageKind::Notebook);
    assert_eq!(pages.rows[1].current, "p2");
    assert_eq!(pages.rows[1].depth, 0);

    assert_eq!(pages.rows[2].id, "p2");
    assert_eq!(pages.rows[2].kind, PageKind::Query);
    assert_eq!(pages.rows[2].depth, 1);

    assert_eq!(pages.rows[3].id, "d1");
    assert_eq!(pages.rows[3].kind, PageKind::Downloader);
    assert_eq!(pages.rows[3].depth, 1);

    // Typed page state replaces the old per-screen JSON blob (WS-15.2.3c).
    assert_eq!(pages.rows[0].query.page_id, "p1");
    assert_eq!(pages.rows[0].query.media.len(), 2);
    assert_eq!(pages.rows[3].downloader.page_id, "d1");
    assert_eq!(pages.rows[3].downloader.kind, "gallery");

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn page_manager_flat_rows_carry_counts_and_active() {
    let (directory, mut state) = testing_state("manager");

    let mut nested = Notebook::new("nb1", "nested");
    let mut p2 = QueryPage::new("p2", "files", "my files");
    p2.hashes = vec![vec![9]];
    nested.children.push(PageNode::Query(p2));
    nested.children.push(PageNode::Placeholder(PlaceholderPage {
        id: "ph1".to_string(),
        title: "old".to_string(),
        message: "gone".to_string(),
        status: String::new(),
    }));

    let mut p1 = QueryPage::new("p1", "files", "my files");
    p1.hashes = vec![vec![1], vec![2], vec![3]];
    state.root.children.clear();
    state.root.children.push(PageNode::Query(p1));
    state.root.children.push(PageNode::Notebook(nested));
    state.active = Some("p2".to_string());

    let tree = page_manager(&state);

    assert_eq!(tree.rows.len(), 4);
    assert_eq!(tree.rows[0].id, "p1");
    assert_eq!(tree.rows[0].kind, PageKind::Query);
    assert_eq!(tree.rows[0].count, 3);
    assert!(!tree.rows[0].active);

    assert_eq!(tree.rows[1].kind, PageKind::Notebook);
    assert_eq!(tree.rows[1].count, 2);
    assert!(!tree.rows[1].active);

    assert_eq!(tree.rows[2].id, "p2");
    assert_eq!(tree.rows[2].depth, 1);
    assert_eq!(tree.rows[2].count, 1);
    assert!(tree.rows[2].active);

    assert_eq!(tree.rows[3].kind, PageKind::Placeholder);
    assert_eq!(tree.rows[3].depth, 1);
    assert_eq!(tree.rows[3].count, 0);

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn query_progress_maps_running_and_idle_import() {
    let mut page = QueryPage::new("p1", "files", "my files");
    page.hashes = vec![vec![1]];
    page.loading = true;

    let import = ImportQueueState {
        running: true,
        done: 2,
        total: 5,
        imported: 1,
        failed: 0,
        message: "importing".to_string(),
    };

    let running = query_progress(&page, Some(&import));
    assert_eq!(running.page_id, "p1");
    assert!(running.loading);
    assert_eq!(running.importing_done, 2);
    assert_eq!(running.importing_total, 5);
    assert_eq!(running.importing_message, "importing");
    assert_eq!(running.count, 1);

    // An idle queue is zero/empty, never JSON null.
    let idle = ImportQueueState {
        running: false,
        done: 2,
        total: 5,
        imported: 0,
        failed: 0,
        message: "stale".to_string(),
    };
    let stopped = query_progress(&page, Some(&idle));
    assert_eq!(stopped.importing_done, 0);
    assert_eq!(stopped.importing_total, 0);
    assert_eq!(stopped.importing_message, "");

    let missing = query_progress(&page, None);
    assert_eq!(missing.importing_done, 0);
    assert_eq!(missing.count, 1);
}

// WS-15.2.2b: the typed query-page seam carries predicates, focused hash,
// media rows and thumbnail options without any JSON.
#[test]
fn typed_query_page_state_carries_media_and_focus() {
    let (directory, state) = testing_state("typed_page");
    let controller = &state.controller;

    let mut page = QueryPage::new("p1", "files", "my files");
    page.query = "red, system:everything".to_string();
    page.hashes = vec![vec![7], vec![8]];
    page.focused = Some(vec![7]);
    page.loading = true;

    let typed = query_page_state(controller, &page);

    assert_eq!(typed.page_id, "p1");
    assert_eq!(typed.query, "red, system:everything");
    assert_eq!(typed.predicates, vec!["red", "system:everything"]);
    assert_eq!(typed.focused, "07");
    assert!(typed.loading);
    assert_eq!(typed.tags, Vec::<String>::new());
    assert_eq!(typed.thumb_scale_percent, 100);
    assert!(!typed.thumb_masonry);
    assert!(!typed.thumb_horizontal);

    assert_eq!(typed.media.len(), 2);
    assert_eq!(typed.media[0].hash, "07");
    assert_eq!(typed.media[1].hash, "08");
    assert!(!typed.media[0].inbox);
    assert!(!typed.media[0].duplicate);
    assert!(!typed.media[0].has_rating);
    assert_eq!(typed.media[0].rating, 0.0);

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn typed_query_page_state_clamps_thumbnail_scale() {
    let (directory, state) = testing_state("typed_scale");
    let controller = &state.controller;

    let mut options = controller.options();
    options.set_integer("thumbnail_scale_percent", 500);
    options.set_boolean("thumbnail_layout_masonry", true);
    options.set_boolean("thumbnail_scroll_horizontal", true);
    controller.replace_options(options);

    let page = QueryPage::new("p1", "files", "my files");
    let typed = query_page_state(controller, &page);

    assert_eq!(typed.thumb_scale_percent, 400);
    assert!(typed.thumb_masonry);
    assert!(typed.thumb_horizontal);

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn typed_media_items_match_legacy_json() {
    let (directory, state) = testing_state("typed_parity");
    let controller = &state.controller;

    let mut page = QueryPage::new("p1", "files", "my files");
    page.hashes = vec![vec![7], vec![8]];

    let typed = crate::gui::app::query::media_items(controller, &page);
    let legacy = crate::gui::app::query::media_json(controller, &page);

    assert_eq!(typed.len(), legacy.len());

    for (item, value) in typed.iter().zip(legacy.iter()) {
        assert_eq!(item.hash, value["hash"].as_str().unwrap());
        assert_eq!(item.thumb, value["thumb"].as_str().unwrap());
        assert_eq!(item.file, value["file"].as_str().unwrap());
        assert_eq!(item.url, value["url"].as_str().unwrap_or_default());
        assert_eq!(item.inbox, value["inbox"].as_bool().unwrap());
        assert_eq!(item.duplicate, value["duplicate"].as_bool().unwrap());
        assert_eq!(item.has_rating, !value["rating"].is_null());
    }

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn typed_selection_state_carries_focus_tags_and_preview() {
    let (directory, state) = testing_state("typed_selection");
    let controller = &state.controller;

    let selection = selection_state(controller, "p1", &[7]);

    assert_eq!(selection.page_id, "p1");
    assert_eq!(selection.hash, "07");
    assert_eq!(selection.tags, Vec::<String>::new());
    assert_eq!(selection.preview_path, "");

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn typed_metadata_state_is_empty_for_unknown_hash() {
    let (directory, state) = testing_state("typed_metadata");
    let controller = &state.controller;

    let metadata = metadata_state(controller, &[9; 32]);

    assert_eq!(
        metadata.hash,
        "0909090909090909090909090909090909090909090909090909090909090909"
    );
    assert!(metadata.tag_services.is_empty());
    assert!(metadata.ratings.is_empty());
    assert!(metadata.notes.is_empty());

    let _ = std::fs::remove_dir_all(directory);
}

// WS-15.2.2b: the typed viewer seam formats the window title and status line
// in Rust; C++ only applies the ready strings.
#[test]
fn typed_viewer_state_formats_title_and_info() {
    let (directory, state) = testing_state("typed_viewer");
    let controller = &state.controller;

    let mut page = QueryPage::new("p1", "files", "my files");
    page.hashes = vec![vec![7], vec![8], vec![9]];

    let viewer = viewer_state(controller, "v1", &page, &[7]);

    assert_eq!(viewer.viewer_id, "v1");
    assert_eq!(viewer.page_id, "p1");
    assert_eq!(viewer.hash, "07");
    assert_eq!(viewer.title, "files - 1/3");
    assert_eq!(viewer.info, "no metadata - file 1/3");
    assert!(!viewer.always_on_top);
    assert!(!viewer.frameless);
    assert!(viewer.zoom_step_percent >= 5 && viewer.zoom_step_percent <= 200);

    let mut options = controller.options();
    options.set_integer("media_viewer_zoom_step_percent", 500);
    controller.replace_options(options);
    let clamped = viewer_state(controller, "v1", &page, &[8]);
    assert_eq!(clamped.zoom_step_percent, 200);
    assert_eq!(clamped.title, "files - 2/3");

    let _ = std::fs::remove_dir_all(directory);
}
