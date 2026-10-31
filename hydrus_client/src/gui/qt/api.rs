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

    /// One flat entry of the main-window menu bar (WS-15.2.2a).
    ///
    /// The menu tree is flattened in Rust with an explicit `depth`:
    /// `0` is a top-level menu (`title`), deeper rows are its items.
    /// A submenu row (`is_submenu`) owns the rows that follow it at
    /// `depth + 1`; a separator row (`is_separator`) only carries `depth`.
    #[derive(Clone, Debug, PartialEq)]
    struct MenuEntry {
        depth: i32,
        title: String,
        label: String,
        action: String,
        target: String,
        payload: String,
        tooltip: String,
        enabled: bool,
        checkable: bool,
        checked: bool,
        is_separator: bool,
        is_submenu: bool,
    }

    /// Main-window menu bar: the whole menu tree as flat rows.
    #[derive(Clone, Debug, PartialEq)]
    struct MenuBar {
        entries: Vec<MenuEntry>,
    }

    /// Page kind for the page/tree seams (WS-15.2.2a). Numeric enum, no
    /// `Option`: every row carries a concrete kind.
    #[derive(Debug)]
    enum PageKind {
        Notebook,
        Query,
        Downloader,
        Subscriptions,
        Duplicates,
        ImportFolders,
        ExportFolders,
        Placeholder,
    }

    /// One page row of the notebook tree (WS-15.2.2a).
    ///
    /// The envelope is typed; `state` keeps the per-screen JSON blob inside
    /// one field (R1 exception: arbitrary per-screen shape) until each
    /// screen converts its own seam in later WS-15.2.2 subtasks.
    #[derive(Clone, Debug, PartialEq)]
    struct PageRow {
        id: String,
        title: String,
        kind: PageKind,
        current: String,
        depth: i32,
        state: String,
    }

    /// Notebook contents for `build_pages`: root current page plus the
    /// whole tree as flat preorder rows.
    #[derive(Clone, Debug, PartialEq)]
    struct Pages {
        current: String,
        rows: Vec<PageRow>,
    }

    /// One row of the manage-pages dialog tree (WS-15.2.2a).
    #[derive(Clone, Debug, PartialEq)]
    struct PageTreeRow {
        id: String,
        title: String,
        kind: PageKind,
        count: i64,
        active: bool,
        depth: i32,
    }

    /// Manage-pages dialog: the whole page tree as flat preorder rows.
    #[derive(Clone, Debug, PartialEq)]
    struct PageTree {
        rows: Vec<PageTreeRow>,
    }

    /// Incremental progress of a file page (WS-15.2.2a).
    ///
    /// C++ currently consumes only `loading` (as the JSON seam did); the
    /// import counters travel typed for the follow-up seams. Absence of an
    /// import is zero/empty, never JSON null.
    #[derive(Clone, Debug, PartialEq)]
    struct PageProgress {
        page_id: String,
        loading: bool,
        importing_done: i64,
        importing_total: i64,
        importing_message: String,
        count: i64,
    }

    /// One thumbnail of a file page (WS-15.2.2b).
    ///
    /// `rating` travels with an explicit `has_rating` flag instead of JSON
    /// null; C++ formats the badge exactly like the old JSON seam
    /// (`QString::number(rating, 'g', 3)`), so the quirk stays in one place.
    #[derive(Clone, Debug, PartialEq)]
    struct MediaItem {
        hash: String,
        thumb: String,
        file: String,
        url: String,
        inbox: bool,
        duplicate: bool,
        has_rating: bool,
        rating: f64,
    }

    /// File page contents for `update_page` (WS-15.2.2b).
    ///
    /// Only fields the C++ side reads. The old JSON seam also carried
    /// `status`, `empty_text`, `metadata`, `count` and `importing`: `status`
    /// travels via `Status`, progress via `PageProgress`, and the metadata
    /// object was ignored by the query page (`Q_UNUSED`), so they are gone.
    #[derive(Clone, Debug, PartialEq)]
    struct QueryPageState {
        page_id: String,
        query: String,
        location: String,
        tag_service: String,
        tag_services: Vec<String>,
        sort: String,
        sort_ascending: bool,
        focused: String,
        tags: Vec<String>,
        predicates: Vec<String>,
        media: Vec<MediaItem>,
        loading: bool,
        thumb_scale_percent: i32,
        thumb_masonry: bool,
        thumb_horizontal: bool,
    }

    /// One display-tag row with its storage tags for the tooltip (WS-15.2.2b).
    ///
    /// Maps do not cross the bridge, so the `display_tag_sources` object of
    /// the old JSON seam becomes a flat vector of pairs.
    #[derive(Clone, Debug, PartialEq)]
    struct TagSource {
        display: String,
        storage: Vec<String>,
    }

    /// One tag service of the metadata panel (WS-15.2.2b).
    #[derive(Clone, Debug, PartialEq)]
    struct TagServiceState {
        key: String,
        name: String,
        tags: Vec<String>,
        display_tags: Vec<String>,
        sources: Vec<TagSource>,
    }

    /// Rating service flavour (WS-15.2.2b). Numeric enum, no `Option`:
    /// every row carries all three value slots and C++ reads the one its
    /// `kind` selects. Absence is zero/empty, never JSON null.
    #[derive(Debug)]
    enum RatingKind {
        Numerical,
        Like,
        IncDec,
    }

    /// One rating service of the metadata panel (WS-15.2.2b).
    #[derive(Clone, Debug, PartialEq)]
    struct RatingState {
        key: String,
        name: String,
        kind: RatingKind,
        value_num: f64,
        value_text: String,
        value_int: i64,
    }

    /// One note of the metadata panel (WS-15.2.2b).
    #[derive(Clone, Debug, PartialEq)]
    struct NoteState {
        name: String,
        text: String,
    }

    /// Metadata panel contents: focused file plus its tag services, rating
    /// services and notes (WS-15.2.2b). Shared by the viewer seam; the query
    /// page never showed it (its old `set_selection` ignored the object).
    #[derive(Clone, Debug, PartialEq)]
    struct MetadataState {
        hash: String,
        tag_services: Vec<TagServiceState>,
        ratings: Vec<RatingState>,
        notes: Vec<NoteState>,
    }

    /// Focus/selection update of a file page (WS-15.2.2b).
    ///
    /// The old JSON seam carried a metadata object that the query page
    /// ignored (`Q_UNUSED(metadata_state)`), so the struct keeps only what
    /// C++ applies: focused hash, selection tags and the preview path.
    #[derive(Clone, Debug, PartialEq)]
    struct SelectionState {
        page_id: String,
        hash: String,
        tags: Vec<String>,
        preview_path: String,
    }

    /// Media viewer contents (WS-15.2.2b).
    ///
    /// Only fields the C++ side reads. The old JSON seam also carried `tags`,
    /// `muted` and `prefetch_*`: none of them was consumed by the viewer
    /// window, so they are gone.
    #[derive(Clone, Debug, PartialEq)]
    struct ViewerState {
        viewer_id: String,
        page_id: String,
        hash: String,
        title: String,
        path: String,
        file_path: String,
        url: String,
        previous_path: String,
        next_path: String,
        info: String,
        always_on_top: bool,
        frameless: bool,
        recenter: bool,
        zoom_step_percent: i32,
        metadata: MetadataState,
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
        fn hydrus_set_menus(window: Pin<&mut HydrusMainWindow>, menus: &MenuBar);
        fn hydrus_set_tag_suggestions(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_show_autocomplete(window: Pin<&mut HydrusMainWindow>, json: &str);
        fn hydrus_set_status(window: Pin<&mut HydrusMainWindow>, status: &StatusBar);
        fn hydrus_build_pages(window: Pin<&mut HydrusMainWindow>, pages: &Pages);
        fn hydrus_update_page(window: Pin<&mut HydrusMainWindow>, page: &QueryPageState);
        fn hydrus_update_page_progress(window: Pin<&mut HydrusMainWindow>, progress: &PageProgress);
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
        fn hydrus_set_selection(window: Pin<&mut HydrusMainWindow>, selection: &SelectionState);
        fn hydrus_show_page_chooser(window: Pin<&mut HydrusMainWindow>, notebook_id: &str);
        fn hydrus_show_page_manager(window: Pin<&mut HydrusMainWindow>, tree: &PageTree);
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
        fn hydrus_open_viewer(window: Pin<&mut HydrusMainWindow>, state: &ViewerState);
        fn hydrus_update_viewer(window: Pin<&mut HydrusMainWindow>, state: &ViewerState);
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
        /// Typed reverse seam (WS-15.2.2a): tab drag & drop between notebooks.
        fn app_move_page(
            window: Pin<&mut HydrusMainWindow>,
            source: &str,
            target: &str,
            position: &str,
        );
        /// Typed reverse seam (WS-15.2.2a): manage-pages dialog close actions.
        fn app_close_pages(window: Pin<&mut HydrusMainWindow>, ids: Vec<String>);
        /// Typed reverse seams (WS-15.2.2b): thumbnail grid selection, focus,
        /// viewer open and page-to-page drag. Same cores as the `"..."` notify
        /// actions, without any JSON payload.
        fn app_set_selection(
            window: Pin<&mut HydrusMainWindow>,
            page_id: &str,
            hashes: Vec<String>,
        );
        fn app_focus_hash(window: Pin<&mut HydrusMainWindow>, page_id: &str, hash: &str);
        fn app_open_viewer(window: Pin<&mut HydrusMainWindow>, page_id: &str, hash: &str);
        fn app_add_hashes(window: Pin<&mut HydrusMainWindow>, page_id: &str, hashes: Vec<String>);
        /// Typed reverse seams (WS-15.2.2b): metadata panel tag/rating/notes
        /// actions. Same cores as `tag_add`/`tag_remove`, `rating_set` and
        /// `notes_manage`, without any JSON payload.
        fn app_tag_edit(
            window: Pin<&mut HydrusMainWindow>,
            hash: &str,
            service: &str,
            tag: &str,
            add: bool,
        );
        fn app_set_rating(
            window: Pin<&mut HydrusMainWindow>,
            hash: &str,
            service: &str,
            action: &str,
            value: f64,
        );
        fn app_manage_notes(window: Pin<&mut HydrusMainWindow>, hash: &str);
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

fn app_move_page(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    source: &str,
    target: &str,
    position: &str,
) {
    crate::gui::app::handle_move_page(window.as_mut(), source, target, position);
}

fn app_close_pages(mut window: Pin<&mut ffi::HydrusMainWindow>, ids: Vec<String>) {
    crate::gui::app::handle_close_pages(window.as_mut(), ids);
}

/// Typed reverse seams (WS-15.2.2b): media/selection/viewer actions from the
/// thumbnail grid, the query page and the metadata panel.
fn app_set_selection(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    hashes: Vec<String>,
) {
    crate::gui::app::handle_set_selection(window.as_mut(), page_id, &hashes);
}

fn app_focus_hash(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str, hash: &str) {
    crate::gui::app::handle_focus_hash(window.as_mut(), page_id, hash);
}

fn app_open_viewer(mut window: Pin<&mut ffi::HydrusMainWindow>, page_id: &str, hash: &str) {
    crate::gui::app::handle_open_viewer(window.as_mut(), page_id, hash);
}

fn app_add_hashes(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    page_id: &str,
    hashes: Vec<String>,
) {
    crate::gui::app::handle_add_hashes(window.as_mut(), page_id, &hashes);
}

fn app_tag_edit(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    hash: &str,
    service: &str,
    tag: &str,
    add: bool,
) {
    crate::gui::app::handle_tag_edit(window.as_mut(), hash, service, tag, add);
}

fn app_set_rating(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    hash: &str,
    service: &str,
    action: &str,
    value: f64,
) {
    crate::gui::app::handle_set_rating(window.as_mut(), hash, service, action, value);
}

fn app_manage_notes(mut window: Pin<&mut ffi::HydrusMainWindow>, hash: &str) {
    crate::gui::app::handle_manage_notes(window.as_mut(), hash);
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
