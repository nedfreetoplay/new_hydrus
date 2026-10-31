//! Typed cxx bridge seam (WS-15.2.1).
//!
//! The single cxx bridge of the Qt facade: shared POD structs plus every
//! `extern "C++"` declaration. One type per seam: Rust builds the struct,
//! `panels.rs` passes it by reference, C++ reads the fields directly. No
//! `QJsonObject` in the typed signatures, no `serde_json` on those seams.
//!
//! Rules for new types here (R1):
//! - only cxx-compatible fields (`String`, integers, `bool`, `f64`,
//!   `Vec<...>` of shared structs); no `Option`, no maps;
//! - missing values are an empty string / zero, never a JSON null;
//! - data of arbitrary shape (page trees, option dicts) stays out: those are
//!   converted per screen in WS-15.2.2, not added here as `&str` payload.

#![allow(dead_code)]

#[cxx::bridge]
// WS-15.2.1: single cxx module with a single `include!("bridge.hpp")`.
// bridge.hpp is a thin aggregator over the topic headers, so one include
// covers every declaration; no ffi_pages/ffi_dialogs split needed.
pub(crate) mod ffi {
    /// Status bar state: one text per label.
    #[derive(Clone, Debug, PartialEq)]
    struct StatusBar {
        message: String,
        media: String,
        bandwidth: String,
        idle: String,
        hydrus_busy: String,
        cpu_busy: String,
        db: String,
    }

    /// About dialog: versions, counts and paths. The window title stays a
    /// C++ literal; there is no `title` field on purpose.
    #[derive(Clone, Debug, PartialEq)]
    struct AboutInfo {
        software_version: u32,
        network_version: u32,
        client_api_version: u32,
        db_version: i64,
        db_dir: String,
        install_dir: String,
        services: i32,
        locations: i32,
        shortcuts: i32,
        api: String,
        sqlite: String,
        rust_crate: String,
        license: String,
        site: String,
        credits: String,
    }

    /// One row of the file viewing statistics table.
    #[derive(Clone, Debug, PartialEq)]
    struct ViewingStatRow {
        canvas: String,
        last_viewed: String,
        views: i64,
        viewtime_secs: i64,
    }

    /// File viewing statistics dialog: hex hash plus media/preview rows.
    #[derive(Clone, Debug, PartialEq)]
    struct ViewingStats {
        hash: String,
        rows: Vec<ViewingStatRow>,
    }

    unsafe extern "C++" {
        include!("bridge.hpp");

        type QApplication;
        type QSplashScreen;
        type HydrusMainWindow;

        fn new_application() -> UniquePtr<QApplication>;
        fn exec_application(application: Pin<&mut QApplication>) -> i32;
        fn quit_application(application: Pin<&mut QApplication>);
        fn qapp_process_events();

        fn new_hydrus_splash() -> UniquePtr<QSplashScreen>;
        fn hydrus_splash_show(splash: Pin<&mut QSplashScreen>);
        fn hydrus_splash_message(splash: Pin<&mut QSplashScreen>, message: &str);
        fn hydrus_splash_finish(splash: Pin<&mut QSplashScreen>);
        fn hydrus_splash_finish_later(splash: Pin<&mut QSplashScreen>, delay_ms: i32);
        fn hydrus_splash_screenshot(splash: Pin<&mut QSplashScreen>, path: &str, delay_ms: i32);

        fn new_hydrus_main_window() -> UniquePtr<HydrusMainWindow>;
        fn build_hydrus_shell(window: Pin<&mut HydrusMainWindow>);
        fn show_hydrus_main_window(window: Pin<&mut HydrusMainWindow>);

        fn hydrus_set_title(window: Pin<&mut HydrusMainWindow>, title: &str);
        fn hydrus_resize_window(window: Pin<&mut HydrusMainWindow>, width: i32, height: i32);
        fn hydrus_set_menus(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_set_tag_suggestions(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_autocomplete(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_set_status(window: Pin<&mut HydrusMainWindow>, status: &StatusBar);
        fn hydrus_build_pages(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_page(window: Pin<&mut HydrusMainWindow>, page_id: &str, json: &str);
        fn hydrus_update_page_progress(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            json: &str,
        );
        fn hydrus_update_downloader(window: Pin<&mut HydrusMainWindow>, page_id: &str, json: &str);
        fn hydrus_update_subscriptions(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            json: &str,
        );
        fn hydrus_show_subscriptions_dialog(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_subscriptions_dialog(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_subscription_editor(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_duplicates(window: Pin<&mut HydrusMainWindow>, page_id: &str, json: &str);
        fn hydrus_update_import_folders(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            json: &str,
        );
        fn hydrus_show_import_folders_dialog(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_import_folders_dialog(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_export_folders(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            json: &str,
        );
        fn hydrus_show_export_folders_dialog(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_export_folders_dialog(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_duplicates_rules(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_duplicate_merge_options(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_import_options(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_duplicate_custom_action(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_auto_resolution_preview(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_duplicate_review(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_duplicate_review(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_tag_relations(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_import_folder(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_local_import_review(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_local_import_tagging(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_close_local_import_review(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_show_repair_file_locations(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_close_repair_file_locations(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_show_export_folder(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_export_review(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_notes(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_set_selection(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            hash: &str,
            tags_json: &str,
            preview_path: &str,
            metadata_json: &str,
        );
        fn hydrus_show_page_chooser(window: Pin<&mut HydrusMainWindow>, notebook_id: &str);
        fn hydrus_show_page_manager(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_command_palette(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_prompt_text(
            window: Pin<&mut HydrusMainWindow>,
            action: &str,
            title: &str,
            label: &str,
            initial: &str,
        );
        fn hydrus_confirm_action(
            window: Pin<&mut HydrusMainWindow>,
            action: &str,
            target: &str,
            title: &str,
            text: &str,
        );
        fn hydrus_show_or_dialog(window: Pin<&mut HydrusMainWindow>, page_id: &str, initial: &str);
        fn hydrus_pick_files(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_prompt_url(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_open_viewer(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_viewer(window: Pin<&mut HydrusMainWindow>, viewer_id: &str, json: &str);
        fn hydrus_close_viewer(window: Pin<&mut HydrusMainWindow>, viewer_id: &str);
        fn hydrus_show_tag_dialog(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_notifications(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_viewing_stats(window: Pin<&mut HydrusMainWindow>, state: &ViewingStats);
        fn hydrus_show_about(window: Pin<&mut HydrusMainWindow>, state: &AboutInfo);
        fn hydrus_export_downloader(window: Pin<&mut HydrusMainWindow>, page_id: &str);
        fn hydrus_show_file_seed_cache(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_gallery_seed_log(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_import_downloader(window: Pin<&mut HydrusMainWindow>, target: &str);
        fn hydrus_show_manage_tags(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_timestamps(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_options(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_services(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_services(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_tag_filter(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_api_permissions(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_api_permissions(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_account_types(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_account_types(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_orphan_files(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_orphan_files(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_database_backup(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_database_backup(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_network(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_update_network(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_start_tick(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_defer_action(
            window: Pin<&mut HydrusMainWindow>,
            action: &str,
            target: &str,
            payload: &str,
            delay_ms: i32,
        );
        fn hydrus_show_message(window: Pin<&mut HydrusMainWindow>, title: &str, text: &str);
        fn hydrus_notify_message(window: Pin<&mut HydrusMainWindow>, message: &str);
        fn hydrus_set_clipboard(window: Pin<&mut HydrusMainWindow>, text: &str);
        fn hydrus_open_path(path: &str);
        fn hydrus_open_url(url: &str);
        fn hydrus_schedule_screenshot(window: Pin<&mut HydrusMainWindow>, path: &str);

        fn qapp_set_application_name(name: &str);
        fn qapp_set_application_display_name(name: &str);
        fn qapp_set_organization_name(name: &str);
        fn qapp_set_style(name: &str);
        fn qapp_set_stylesheet(stylesheet: &str);
        fn qapp_quit();
    }

    extern "Rust" {
        fn app_action(
            window: Pin<&mut HydrusMainWindow>,
            action: &str,
            target: &str,
            payload: &str,
        );
        fn app_tick(window: Pin<&mut HydrusMainWindow>);
        fn hydrus_validate_export_phrase(phrase: &str) -> String;
        fn hydrus_static_dir() -> String;
    }
}

use std::pin::Pin;

fn app_action(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    action: &str,
    target: &str,
    payload: &str,
) {
    crate::gui::app::handle_action(window.as_mut(), action, target, payload);
}

fn app_tick(mut window: Pin<&mut ffi::HydrusMainWindow>) {
    crate::gui::app::handle_tick(window.as_mut());
}

/// Returns the Python `VetoException` text for an invalid export filename
/// phrase, or an empty string when the phrase parses.
fn hydrus_validate_export_phrase(phrase: &str) -> String {
    crate::gui::app::export::validate_phrase(phrase)
        .err()
        .unwrap_or_default()
}

fn hydrus_static_dir() -> String {
    crate::gui::app::static_assets::resolve_static_dir()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
}
