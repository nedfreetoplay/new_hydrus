//! JSON rendering of application state for the Qt layer.

use serde_json::{Value, json};

use hydrus_core::hydrus_constants::ServiceType;

use super::model::{
    AppState, DownloaderKind, DownloaderPage, DuplicatesPage, ImportQueueState, PageNode,
    QueryPage,
};
use super::query;
use crate::gui::qt::api::ffi::{
    AboutInfo, MenuBar, MenuEntry, MetadataState, NoteState, PageKind, PageProgress,
    PageRow, PageTree, PageTreeRow, Pages, QueryPageState, RatingKind, RatingState, SelectionState,
    StatusBar, TagServiceState, TagSource, UiAction, ViewerState, ViewingStatRow, ViewingStats,
};

fn menu_title(entries: &mut Vec<MenuEntry>, title: &str) {
    entries.push(MenuEntry {
        depth: 0,
        title: title.to_string(),
        label: String::new(),
        action: UiAction::None,
        target: String::new(),
        payload: String::new(),
        tooltip: String::new(),
        enabled: true,
        checkable: false,
        checked: false,
        is_separator: false,
        is_submenu: false,
    });
}

fn menu_separator(entries: &mut Vec<MenuEntry>, depth: i32) {
    entries.push(MenuEntry {
        depth,
        title: String::new(),
        label: String::new(),
        action: UiAction::None,
        target: String::new(),
        payload: String::new(),
        tooltip: String::new(),
        enabled: true,
        checkable: false,
        checked: false,
        is_separator: true,
        is_submenu: false,
    });
}

fn menu_submenu(entries: &mut Vec<MenuEntry>, depth: i32, label: &str) {
    entries.push(MenuEntry {
        depth,
        title: String::new(),
        label: label.to_string(),
        action: UiAction::None,
        target: String::new(),
        payload: String::new(),
        tooltip: String::new(),
        enabled: true,
        checkable: false,
        checked: false,
        is_separator: false,
        is_submenu: true,
    });
}

#[allow(clippy::too_many_arguments)]
fn menu_action(
    entries: &mut Vec<MenuEntry>,
    depth: i32,
    label: &str,
    action: &str,
    target: &str,
    payload: &str,
    tooltip: &str,
    enabled: bool,
) {
    entries.push(MenuEntry {
        depth,
        title: String::new(),
        label: label.to_string(),
        action: crate::gui::app::action_from_str(action),
        target: target.to_string(),
        payload: payload.to_string(),
        tooltip: tooltip.to_string(),
        enabled,
        checkable: false,
        checked: false,
        is_separator: false,
        is_submenu: false,
    });
}

fn menu_toggle(
    entries: &mut Vec<MenuEntry>,
    depth: i32,
    label: &str,
    action: &str,
    tooltip: &str,
    checked: bool,
) {
    entries.push(MenuEntry {
        depth,
        title: String::new(),
        label: label.to_string(),
        action: crate::gui::app::action_from_str(action),
        target: String::new(),
        payload: String::new(),
        tooltip: tooltip.to_string(),
        enabled: true,
        checkable: true,
        checked,
        is_separator: false,
        is_submenu: false,
    });
}

fn menu_disabled(entries: &mut Vec<MenuEntry>, depth: i32, label: &str) {
    menu_action(entries, depth, label, "", "", "", "", false);
}

/// Flat, typed main-window menu bar (WS-15.2.2a): the same tree the JSON
/// seam carried, with `depth` marking submenus. Predicates (`enabled`,
/// `checked`, hidden `&pending`) already lived here in Rust; C++ only
/// rebuilds `QMenu`s from the rows.
pub fn menus(state: &AppState) -> MenuBar {
    let services = state.controller.services();
    let mut entries = Vec::new();

    menu_title(&mut entries, "&file");
    menu_action(
        &mut entries,
        1,
        "import files...",
        "import_files",
        "",
        "",
        "Add new files to the database.",
        true,
    );
    menu_action(
        &mut entries,
        1,
        "import options...",
        "import_options",
        "",
        "",
        "Choose file filters, destinations, tags, notes and presentation.",
        true,
    );
    menu_action(
        &mut entries,
        1,
        "import url...",
        "import_url",
        "",
        "",
        "Download one file from a URL and import it.",
        true,
    );
    menu_action(
        &mut entries,
        1,
        "manage import folders\u{2026}",
        "manage_import_folders",
        "",
        "",
        "Manage folders from which the client can automatically import.",
        true,
    );
    menu_submenu(&mut entries, 1, "import folders");
    import_folder_menu_entries(state, &mut entries, 2);
    menu_submenu(&mut entries, 1, "export folders");
    export_folder_menu_entries(state, &mut entries, 2);
    menu_separator(&mut entries, 1);
    menu_submenu(&mut entries, 1, "open");
    menu_action(
        &mut entries,
        2,
        "installation directory",
        "open_install_dir",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        2,
        "database directory",
        "open_db_dir",
        "",
        "",
        "",
        true,
    );
    menu_separator(&mut entries, 1);
    menu_action(&mut entries, 1, "options...", "options", "", "", "", true);
    menu_separator(&mut entries, 1);
    menu_action(&mut entries, 1, "exit", "exit", "", "", "", true);

    menu_title(&mut entries, "&undo");
    menu_action(
        &mut entries,
        1,
        "undo",
        "undo",
        "",
        "",
        "Restore the most recently closed page.",
        !state.closed_pages.is_empty(),
    );

    menu_title(&mut entries, "&pages");
    menu_action(&mut entries, 1, "new page...", "new_page", "", "", "", true);
    menu_submenu(&mut entries, 1, "download");
    menu_action(
        &mut entries,
        2,
        "new url download page",
        "new_urls",
        "",
        "",
        "Download direct file URLs.",
        true,
    );
    menu_action(
        &mut entries,
        2,
        "new watcher page",
        "new_watcher",
        "",
        "",
        "Watch a page for new files.",
        true,
    );
    menu_action(
        &mut entries,
        2,
        "new gallery page",
        "new_gallery",
        "",
        "",
        "Parse a gallery page and download its files.",
        true,
    );
    menu_action(
        &mut entries,
        2,
        "new simple downloader page",
        "new_simple",
        "",
        "",
        "Parse page links and download the files they point to.",
        true,
    );
    menu_action(&mut entries, 1, "refresh", "refresh_page", "", "", "", true);
    menu_separator(&mut entries, 1);
    menu_action(
        &mut entries,
        1,
        "back",
        "history_back",
        "",
        "",
        "",
        state.history.can_back(),
    );
    menu_action(
        &mut entries,
        1,
        "forward",
        "history_forward",
        "",
        "",
        "",
        state.history.can_forward(),
    );
    menu_separator(&mut entries, 1);
    menu_action(
        &mut entries,
        1,
        "undo search",
        "undo_search",
        "",
        "",
        "",
        state
            .active_query()
            .is_some_and(|page| !page.query_undo.is_empty()),
    );
    menu_action(
        &mut entries,
        1,
        "redo search",
        "redo_search",
        "",
        "",
        "",
        state
            .active_query()
            .is_some_and(|page| !page.query_redo.is_empty()),
    );
    menu_submenu(&mut entries, 1, "recent searches");
    recent_search_entries(state, &mut entries, 2);
    menu_separator(&mut entries, 1);
    menu_action(&mut entries, 1, "new page of pages", "new_notebook", "", "", "", true);
    menu_action(
        &mut entries,
        1,
        "new duplicates processing page",
        "new_duplicates",
        "",
        "",
        "",
        true,
    );
    menu_separator(&mut entries, 1);
    menu_action(&mut entries, 1, "manage pages...", "page_manager", "", "", "", true);
    menu_action(&mut entries, 1, "duplicate page", "duplicate_page", "", "", "", true);
    menu_separator(&mut entries, 1);
    menu_action(&mut entries, 1, "close page", "close_page", "", "", "", true);
    menu_action(
        &mut entries,
        1,
        "close other pages",
        "close_other_pages",
        "",
        "",
        "",
        true,
    );
    menu_separator(&mut entries, 1);
    menu_submenu(&mut entries, 1, "favourite searches");
    favourite_menu_entries(state, &mut entries, 2);
    menu_separator(&mut entries, 1);
    menu_submenu(&mut entries, 1, "sessions");
    session_menu_entries(state, &mut entries, 2);

    menu_title(&mut entries, "&database");
    menu_action(
        &mut entries,
        1,
        "database directory",
        "open_db_dir",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        1,
        "show database info",
        "db_info",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        1,
        "back up database...",
        "db_backup_open",
        "",
        "",
        "Copy client.db to the configured backup directory.",
        true,
    );
    menu_separator(&mut entries, 1);
    menu_submenu(&mut entries, 1, "maintenance");
    menu_action(&mut entries, 2, "check database", "db_check", "", "", "", true);
    menu_action(&mut entries, 2, "analyze database", "db_analyze", "", "", "", true);
    menu_action(&mut entries, 2, "vacuum database", "db_vacuum", "", "", "", true);
    menu_action(
        &mut entries,
        2,
        "review vacuum data",
        "db_vacuum_info",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        2,
        "review deferred deletes",
        "db_deferred_deletes",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        2,
        "fix missing archive times",
        "db_fix_archive_times",
        "",
        "",
        "",
        true,
    );
    menu_separator(&mut entries, 2);
    menu_action(
        &mut entries,
        2,
        "review file maintenance jobs",
        "db_file_maintenance",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        2,
        "run file maintenance jobs",
        "db_run_file_maintenance",
        "",
        "",
        "",
        true,
    );
    menu_separator(&mut entries, 2);
    menu_action(
        &mut entries,
        2,
        "clear orphan files...",
        "orphan_files_scan",
        "",
        "",
        "",
        true,
    );
    menu_separator(&mut entries, 2);
    menu_action(
        &mut entries,
        2,
        "review local imports...",
        "import_files",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        2,
        "repair file locations...",
        "repair_file_locations",
        "",
        "",
        "",
        true,
    );
    menu_separator(&mut entries, 2);
    menu_action(&mut entries, 2, "how boned am I?", "db_stats", "", "", "", true);

    menu_title(&mut entries, "&network");
    menu_action(
        &mut entries,
        1,
        "manage network...",
        "options_network",
        "",
        "",
        "",
        true,
    );
    menu_toggle(
        &mut entries,
        1,
        "pause all new network traffic",
        "pause_network",
        "",
        state.controller.network_paused(),
    );
    menu_action(
        &mut entries,
        1,
        "subscriptions\u{2026}",
        "manage_subscriptions",
        "",
        "",
        "Change the queries you want the client to regularly import from.",
        true,
    );
    menu_separator(&mut entries, 1);
    menu_action(
        &mut entries,
        1,
        "export downloader definition...",
        "downloader_export",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        1,
        "import downloader definition...",
        "downloader_import",
        "",
        "",
        "",
        true,
    );

    menu_title(&mut entries, "&services");
    service_menu_entries(&services, &mut entries);

    menu_title(&mut entries, "&tags");
    menu_action(
        &mut entries,
        1,
        "manage tags...",
        "manage_tags_active",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        1,
        "manage notes...",
        "manage_notes_active",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        1,
        "edit timestamps...",
        "edit_timestamps",
        "",
        "",
        "",
        true,
    );
    menu_separator(&mut entries, 1);
    menu_action(&mut entries, 1, "copy tags", "copy_tags", "", "", "", true);
    menu_action(
        &mut entries,
        1,
        "paste tags...",
        "paste_tags_prompt",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        1,
        "apply tags to selection...",
        "apply_tags_selection",
        "",
        "",
        "",
        true,
    );
    menu_separator(&mut entries, 1);
    menu_action(
        &mut entries,
        1,
        "duplicate auto-resolution...",
        "duplicates_rules",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        1,
        "review duplicate pending decisions...",
        "duplicates_review",
        "",
        "",
        "",
        true,
    );
    menu_separator(&mut entries, 1);
    menu_action(
        &mut entries,
        1,
        "manage tag siblings...",
        "tag_siblings",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        1,
        "manage tag parents...",
        "tag_parents",
        "",
        "",
        "",
        true,
    );

    // Python hides the pending menu unless a repository service exists
    // (`has_pending_services` in ClientGUI).
    let has_pending_services = !services
        .get_services(Some(&[
            ServiceType::TagRepository,
            ServiceType::FileRepository,
            ServiceType::Ipfs,
        ]))
        .is_empty();

    if has_pending_services {
        menu_title(&mut entries, "&pending");
        menu_action(&mut entries, 1, "pending uploads", "pending", "", "", "", false);
    }

    menu_title(&mut entries, "&help");
    menu_action(&mut entries, 1, "documentation", "docs", "", "", "", true);
    menu_action(
        &mut entries,
        1,
        "popup messages...",
        "notifications",
        "",
        "",
        "",
        true,
    );
    menu_action(
        &mut entries,
        1,
        "command palette...",
        "command_palette",
        "",
        "",
        "Ctrl+K",
        true,
    );
    menu_action(&mut entries, 1, "about", "about", "", "", "", true);
    menu_separator(&mut entries, 1);
    menu_submenu(&mut entries, 1, "debug");
    menu_action(
        &mut entries,
        2,
        "show database info",
        "db_info",
        "",
        "",
        "",
        true,
    );

    MenuBar { entries }
}

fn service_menu_entries(
    services: &crate::services::ServicesManager,
    entries: &mut Vec<MenuEntry>,
) {
    for service in services.get_services(None) {
        menu_action(
            entries,
            1,
            &service.name,
            "service_info",
            &service.name,
            "",
            "show information about this service",
            true,
        );
    }

    let tag_filter_services = services.get_services(Some(&[
        ServiceType::TagRepository,
        ServiceType::LocalTag,
    ]));

    if !tag_filter_services.is_empty() {
        menu_separator(entries, 1);
        menu_submenu(entries, 1, "tag filters");
        for service in tag_filter_services {
            menu_action(
                entries,
                2,
                &service.name,
                "tag_filter_open",
                "",
                &query::hex(&service.service_key),
                "edit the tag filter and purge tags",
                true,
            );
        }
    }

    menu_separator(entries, 1);
    menu_action(
        entries,
        1,
        "manage services...",
        "manage_services",
        "",
        "",
        "local services are managed here",
        true,
    );
    menu_action(
        entries,
        1,
        "client api permissions...",
        "api_permissions",
        "",
        "",
        "manage Client API access keys and permissions",
        true,
    );
}


fn import_folder_menu_entries(
    state: &AppState,
    entries: &mut Vec<MenuEntry>,
    depth: i32,
) {
    menu_action(entries, depth, "check all", "import_folder_check_all", "", "", "", true);
    menu_separator(entries, depth);

    for folder in &state.import_folders {
        menu_action(
            entries,
            depth,
            &folder.name,
            "import_folder_check",
            "",
            &folder.name,
            "",
            true,
        );
    }

    if state.import_folders.is_empty() {
        menu_disabled(entries, depth, "no import folders");
    }
}

fn export_folder_menu_entries(
    state: &AppState,
    entries: &mut Vec<MenuEntry>,
    depth: i32,
) {
    menu_action(
        entries,
        depth,
        "manage export folders\u{2026}",
        "manage_export_folders",
        "",
        "",
        "Manage folders to which the client can automatically export.",
        true,
    );
    menu_action(
        entries,
        depth,
        "run all export folders",
        "export_folder_run_all",
        "",
        "",
        "",
        !state.export_folders.is_empty(),
    );
    menu_separator(entries, depth);

    for folder in &state.export_folders {
        menu_action(
            entries,
            depth,
            &folder.name,
            "export_folder_run",
            "",
            &folder.name,
            &folder.status,
            !folder.running && !folder.paused,
        );
    }

    if state.export_folders.is_empty() {
        menu_disabled(entries, depth, "no export folders");
    }
}

fn recent_search_entries(state: &AppState, entries: &mut Vec<MenuEntry>, depth: i32) {
    for query in state.search_history.iter().take(15) {
        menu_action(
            entries,
            depth,
            query,
            "apply_search_history",
            query,
            "",
            "",
            true,
        );
    }

    if state.search_history.is_empty() {
        menu_disabled(entries, depth, "no recent searches");
    }
}

fn favourite_menu_entries(state: &AppState, entries: &mut Vec<MenuEntry>, depth: i32) {
    for favourite in &state.favourites {
        menu_action(
            entries,
            depth,
            &format!("{} — {}", favourite.name, favourite.query),
            "favourite_apply",
            &favourite.name,
            "",
            "",
            true,
        );
    }

    if state.favourites.is_empty() {
        menu_disabled(entries, depth, "no favourite searches");
    }

    menu_separator(entries, depth);
    menu_action(
        entries,
        depth,
        "save current search...",
        "favourite_save",
        "",
        "",
        "Store the current query, file domain and tag service.",
        true,
    );

    menu_submenu(entries, depth, "delete");

    for favourite in &state.favourites {
        menu_action(
            entries,
            depth + 1,
            &favourite.name,
            "favourite_delete",
            &favourite.name,
            "",
            "",
            true,
        );
    }

    if state.favourites.is_empty() {
        menu_disabled(entries, depth + 1, "no favourite searches");
    }
}

fn session_menu_entries(state: &AppState, entries: &mut Vec<MenuEntry>, depth: i32) {
    menu_action(
        entries,
        depth,
        "save current as...",
        "session_save_as",
        "",
        "",
        "Save the open pages under a new session name.",
        true,
    );
    menu_action(
        entries,
        depth,
        "backup current",
        "session_backup",
        "",
        "",
        "Copy the open pages into the backup folder.",
        true,
    );
    menu_separator(entries, depth);

    let sessions = super::sessions::list(state);
    let empty = sessions.is_empty();

    menu_submenu(entries, depth, "load");
    for session in &sessions {
        let label = format!("{} ({} pages)", session.name, session.pages);
        menu_action(entries, depth + 1, &label, "session_load", &session.name, "", "", true);
    }
    if empty {
        menu_disabled(entries, depth + 1, "no saved sessions");
    }

    menu_submenu(entries, depth, "append");
    for session in &sessions {
        let label = format!("{} ({} pages)", session.name, session.pages);
        menu_action(
            entries,
            depth + 1,
            &label,
            "session_append",
            &session.name,
            "",
            "",
            true,
        );
    }
    if empty {
        menu_disabled(entries, depth + 1, "no saved sessions");
    }

    menu_submenu(entries, depth, "append backup");
    let backups = super::sessions::list_backups(state);
    for backup in &backups {
        menu_action(
            entries,
            depth + 1,
            &format!("{} ({} pages)", backup.name, backup.pages),
            "session_append_backup",
            &backup.name,
            "",
            "",
            true,
        );
    }
    if backups.is_empty() {
        menu_disabled(entries, depth + 1, "no backups");
    }

    menu_submenu(entries, depth, "delete");
    for session in &sessions {
        let label = format!("{} ({} pages)", session.name, session.pages);
        menu_action(
            entries,
            depth + 1,
            &label,
            "session_delete",
            &session.name,
            "",
            "",
            true,
        );
    }
    if empty {
        menu_disabled(entries, depth + 1, "no saved sessions");
    }
}

pub fn notifications_json(state: &AppState) -> String {
    let entries: Vec<Value> = state
        .notifications
        .entries()
        .iter()
        .rev()
        .map(|notification| {
            json!({
                "time": hydrus_core::hydrus_time::timestamp_to_pretty_time(
                    Some(notification.timestamp as f64),
                    false,
                    true,
                ),
                "level": notification.level,
                "text": notification.text,
            })
        })
        .collect();

    json!({"count": state.notifications.len(), "entries": entries}).to_string()
}

/// Manage-pages dialog tree as flat preorder rows (WS-15.2.2a).
/// `count` is the per-kind item count the old JSON seam carried; `active`
/// marks the selected page (rendered bold).
pub fn page_manager(state: &AppState) -> PageTree {
    let mut rows = Vec::new();
    manager_rows(state, &state.root, 0, &mut rows);
    PageTree { rows }
}

fn manager_rows(
    state: &AppState,
    notebook: &super::model::Notebook,
    depth: i32,
    rows: &mut Vec<PageTreeRow>,
) {
    for child in &notebook.children {
        manager_item_rows(state, child, depth, rows);
    }
}

fn manager_item_rows(
    state: &AppState,
    node: &PageNode,
    depth: i32,
    rows: &mut Vec<PageTreeRow>,
) {
    let active = state.active.as_deref() == Some(node.id());
    match node {
        PageNode::Notebook(notebook) => {
            rows.push(PageTreeRow {
                id: notebook.id.clone(),
                title: notebook.title.clone(),
                kind: PageKind::Notebook,
                count: notebook.children.len() as i64,
                active: state.active.as_deref() == Some(notebook.id.as_str()),
                depth,
            });
            manager_rows(state, notebook, depth + 1, rows);
        }
        PageNode::Query(page) => rows.push(PageTreeRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::Query,
            count: page.hashes.len() as i64,
            active,
            depth,
        }),
        PageNode::Downloader(page) => rows.push(PageTreeRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::Downloader,
            count: page.results.len() as i64,
            active,
            depth,
        }),
        PageNode::Subscriptions(page) => rows.push(PageTreeRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::Subscriptions,
            count: state.subscriptions.len() as i64,
            active,
            depth,
        }),
        PageNode::Duplicates(page) => rows.push(PageTreeRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::Duplicates,
            count: page.pairs.len() as i64,
            active,
            depth,
        }),
        PageNode::ImportFolders(page) => rows.push(PageTreeRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::ImportFolders,
            count: state.import_folders.len() as i64,
            active,
            depth,
        }),
        PageNode::ExportFolders(page) => rows.push(PageTreeRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::ExportFolders,
            count: state.export_folders.len() as i64,
            active,
            depth,
        }),
        PageNode::Placeholder(page) => rows.push(PageTreeRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::Placeholder,
            count: 0,
            active,
            depth,
        }),
    }
}

/// Notebook contents for `build_pages` as flat preorder rows (WS-15.2.2a).
/// The envelope (id/title/kind/current/depth) is typed; `state` keeps the
/// per-screen JSON blob inside one field (R1 exception) until each screen
/// converts its own seam in later WS-15.2.2 subtasks.
pub fn pages(state: &AppState) -> Pages {
    let mut rows = Vec::new();
    pages_rows(state, &state.root, 0, &mut rows);
    Pages {
        current: state.root.current.clone().unwrap_or_default(),
        rows,
    }
}

fn pages_rows(
    state: &AppState,
    notebook: &super::model::Notebook,
    depth: i32,
    rows: &mut Vec<PageRow>,
) {
    for child in &notebook.children {
        page_row(state, child, depth, rows);
    }
}

fn page_row(state: &AppState, node: &PageNode, depth: i32, rows: &mut Vec<PageRow>) {
    match node {
        PageNode::Query(page) => rows.push(PageRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::Query,
            current: String::new(),
            depth,
            state: serde_json::to_string(&query_state(
                &state.controller,
                page,
                Some(&state.import_queue),
            ))
            .unwrap_or_else(|_| "{}".to_string()),
        }),
        PageNode::Notebook(notebook) => {
            rows.push(PageRow {
                id: notebook.id.clone(),
                title: notebook.title.clone(),
                kind: PageKind::Notebook,
                current: notebook.current.clone().unwrap_or_default(),
                depth,
                state: String::new(),
            });
            pages_rows(state, notebook, depth + 1, rows);
        }
        PageNode::Downloader(page) => rows.push(PageRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::Downloader,
            current: String::new(),
            depth,
            state: downloader_json(&state.controller, page),
        }),
        PageNode::Subscriptions(page) => rows.push(PageRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::Subscriptions,
            current: String::new(),
            depth,
            state: subscriptions_json(state, &page.id),
        }),
        PageNode::Duplicates(page) => rows.push(PageRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::Duplicates,
            current: String::new(),
            depth,
            state: duplicates_json(&state.controller, page),
        }),
        PageNode::ImportFolders(page) => rows.push(PageRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::ImportFolders,
            current: String::new(),
            depth,
            state: super::import_folders::rules_json(state, &page.id),
        }),
        PageNode::ExportFolders(page) => rows.push(PageRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::ExportFolders,
            current: String::new(),
            depth,
            state: super::export::folders_json(state, &page.id),
        }),
        PageNode::Placeholder(page) => rows.push(PageRow {
            id: page.id.clone(),
            title: page.title.clone(),
            kind: PageKind::Placeholder,
            current: String::new(),
            depth,
            state: serde_json::to_string(&serde_json::json!({"message": page.message}))
                .unwrap_or_else(|_| "{}".to_string()),
        }),
    }
}

pub fn query_state(
    controller: &crate::controller::Controller,
    page: &QueryPage,
    import: Option<&super::model::ImportQueueState>,
) -> Value {
    let focused = page
        .focused
        .as_ref()
        .map(|hash| query::hex(hash))
        .unwrap_or_default();

    let tags = page
        .focused
        .as_ref()
        .map(|hash| query::selection_tags(controller, hash))
        .unwrap_or_default();

    let metadata = page
        .focused
        .as_ref()
        .map(|hash| super::metadata::metadata_json(controller, hash))
        .unwrap_or_else(|| json!({"tag_services": [], "ratings": [], "notes": []}));

    let options = controller.options();
    let scale = options
        .get_integer("thumbnail_scale_percent")
        .clamp(25, 400);
    let importing = importing_json(import);

    json!({
        "query": page.query,
        "location": page.location,
        "tag_service": page.tag_service,
        "tag_services": query::local_tag_service_names(controller),
        "sort": page.sort,
        "sort_ascending": page.sort_ascending,
        "status": page.status,
        "empty_text": page.empty_text,
        "focused": focused,
        "tags": tags,
        "predicates": page
            .query
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>(),
        "metadata": metadata,
        "media": query::media_json(controller, page),
        "count": page.hashes.len(),
        "loading": page.loading,
        "importing": importing,
        "thumbnails": {
            "scale_percent": scale,
            "masonry": options.get_boolean("thumbnail_layout_masonry"),
            "horizontal": options.get_boolean("thumbnail_scroll_horizontal"),
        },
    })
}

fn importing_json(import: Option<&super::model::ImportQueueState>) -> Value {
    import
        .filter(|queue| queue.running)
        .map(|queue| {
            json!({
                "done": queue.done,
                "total": queue.total,
                "message": queue.message,
            })
        })
        .unwrap_or(Value::Null)
}

/// Small incremental payload sent while imports run, so the results panel can
/// show progress without rebuilding the whole thumbnail list.
pub fn query_progress(
    page: &QueryPage,
    import: Option<&super::model::ImportQueueState>,
) -> PageProgress {
    let running = import.filter(|queue| queue.running);
    PageProgress {
        page_id: page.id.clone(),
        loading: page.loading,
        importing_done: running.map(|queue| queue.done as i64).unwrap_or(0),
        importing_total: running.map(|queue| queue.total as i64).unwrap_or(0),
        importing_message: running.map(|queue| queue.message.clone()).unwrap_or_default(),
        count: page.hashes.len() as i64,
    }
}

pub fn maintenance_json(state: &AppState) -> String {
    let counts = state.controller.maintenance().get_job_counts();
    let total: i64 = counts.values().sum();

    if counts.is_empty() {
        return "no file maintenance jobs are queued".to_string();
    }

    let mut text = format!("{total} file maintenance job(s) queued\n");

    for (job_type, count) in counts {
        text.push_str(&format!(
            "\n{} — {count}",
            crate::files::maintenance::regen_file_enum_to_str(job_type)
        ));
    }

    text
}

pub fn about(state: &AppState) -> AboutInfo {
    let controller = &state.controller;
    let db = controller.db().db();
    let sqlite: String = db
        .query_one("SELECT sqlite_version()", &[], |row| row.get(0))
        .ok()
        .flatten()
        .unwrap_or_default();
    let install_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.display().to_string()))
        .unwrap_or_default();

    AboutInfo {
        software_version: hydrus_core::hydrus_constants::SOFTWARE_VERSION,
        network_version: hydrus_core::hydrus_constants::NETWORK_VERSION,
        client_api_version: hydrus_core::hydrus_constants::CLIENT_API_VERSION,
        db_version: controller.db().get_version().unwrap_or(0),
        db_dir: controller.db_dir().display().to_string(),
        install_dir,
        services: controller.services().get_num_services() as i32,
        locations: controller.files_manager().locations().len() as i32,
        shortcuts: controller.shortcuts().len() as i32,
        api: controller
            .api_status()
            .unwrap_or_else(|| "stopped".to_string()),
        sqlite,
        rust_crate: env!("CARGO_PKG_VERSION").to_string(),
        license: "Hydrus is free software, released under the WTFPL.".to_string(),
        site: "https://hydrusnetwork.github.io/hydrus/".to_string(),
        credits: "Hydrus Network by the Hydrus developer and contributors.".to_string(),
    }
}

/// Status-bar media text for a query page: `3/12 - ` from the focused file,
/// `N selected - ` when several files are selected (R2: pure, tested).
pub fn query_page_media_text(page: &QueryPage) -> String {
    let position = page
        .focused
        .as_ref()
        .map(|hash| {
            let index = query::viewer_position(page, hash);
            format!("{}/{} - ", index + 1, page.hashes.len())
        })
        .unwrap_or_default();

    if page.selected.len() > 1 {
        format!("{} selected - {}", page.selected.len(), page.status)
    } else {
        format!("{position}{}", page.status)
    }
}

/// Empty-state media text for a downloader page (R2: pure, tested).
pub fn downloader_page_media_text(page: &DownloaderPage) -> String {
    if page.kind == DownloaderKind::Watcher && !page.url.is_empty() {
        "no files for this watcher and its publishing settings".to_string()
    } else if page.kind == DownloaderKind::Watcher {
        "no highlighted watcher".to_string()
    } else if page.kind == DownloaderKind::Gallery {
        "no highlighted query".to_string()
    } else if page.queue.is_empty() && page.results.is_empty() {
        "empty page".to_string()
    } else {
        format!("{} files", page.results.len())
    }
}

/// `done/total` while the import queue runs, otherwise `title: done/total`
/// of the running downloader, otherwise empty (R2: pure, tested).
pub fn busy_text(import: &ImportQueueState, running: Option<(&str, usize, usize)>) -> String {
    if import.running {
        format!("{}/{}", import.done, import.total)
    } else if let Some((title, done, total)) = running {
        format!("{title}: {done}/{total}")
    } else {
        String::new()
    }
}

/// `"idle"` when nothing is busy, otherwise empty (R2: pure, tested).
pub fn idle_text(busy: &str) -> String {
    if busy.is_empty() {
        "idle".to_string()
    } else {
        String::new()
    }
}

/// `"network paused"` when paused, otherwise the `0B` placeholder the Python
/// client shows before bandwidth accounting (R2: pure, tested).
pub fn bandwidth_text(network_paused: bool) -> String {
    if network_paused {
        "network paused".to_string()
    } else {
        "0B".to_string()
    }
}

pub fn status(state: &AppState) -> StatusBar {
    let media = match state.active_query() {
        Some(page) => query_page_media_text(page),
        None => match state.active_downloader() {
            Some(page) => downloader_page_media_text(page),
            None => String::new(),
        },
    };

    let running = running_downloader(state);
    let busy = busy_text(
        &state.import_queue,
        running
            .as_ref()
            .map(|(title, done, total)| (title.as_str(), *done, *total)),
    );

    StatusBar {
        message: state.message.clone(),
        media,
        bandwidth: bandwidth_text(state.controller.network_paused()),
        idle: idle_text(&busy),
        hydrus_busy: if busy.is_empty() {
            String::new()
        } else {
            "working".to_string()
        },
        cpu_busy: String::new(),
        db: String::new(),
    }
}

fn running_downloader(state: &AppState) -> Option<(String, usize, usize)> {
    fn visit(notebook: &super::model::Notebook, best: &mut Option<(String, usize, usize)>) {
        for child in &notebook.children {
            match child {
                PageNode::Downloader(page) if page.running => {
                    *best = Some((
                        page.title.clone(),
                        page.imported + page.failed,
                        page.queue.len(),
                    ));
                }
                PageNode::Notebook(inner) => visit(inner, best),
                _ => {}
            }
        }
    }

    let mut best = None;
    visit(&state.root, &mut best);

    best.or_else(|| {
        state
            .subscriptions
            .iter()
            .find(|subscription| subscription.running)
            .map(|subscription| {
                (
                    subscription.name.clone(),
                    subscription.imported + subscription.failed,
                    0,
                )
            })
    })
}

pub fn viewing_stats(controller: &crate::controller::Controller, hash: &[u8]) -> ViewingStats {
    let Some(hash_id) = query::hash_id_for(controller, hash) else {
        return ViewingStats {
            hash: query::hex(hash),
            rows: Vec::new(),
        };
    };

    let db = controller.db().db();
    let mut rows = Vec::new();

    for (canvas_type, name) in [(0i64, "media"), (1i64, "preview")] {
        let stats = crate::db::files_viewing_stats::get_viewing_stats(db, hash_id, canvas_type)
            .unwrap_or_default();

        rows.push(ViewingStatRow {
            canvas: name.to_string(),
            last_viewed: hydrus_core::hydrus_time::timestamp_to_pretty_time(
                stats.last_viewed_timestamp_ms.map(|value| value as f64 / 1000.0),
                false,
                true,
            ),
            views: stats.views,
            viewtime_secs: stats.viewtime_ms / 1000,
        });
    }

    ViewingStats {
        hash: query::hex(hash),
        rows,
    }
}

#[cfg(test)]
mod tests;

/// Typed file-page contents for `UiCommand::UpdatePage` (WS-15.2.2b).
/// Same source data as `query_state` (which stays for the page-tree blob),
/// minus the fields C++ never reads.
pub fn query_page_state(
    controller: &crate::controller::Controller,
    page: &QueryPage,
) -> QueryPageState {
    let focused = page
        .focused
        .as_ref()
        .map(|hash| query::hex(hash))
        .unwrap_or_default();

    let tags = page
        .focused
        .as_ref()
        .map(|hash| query::selection_tags(controller, hash))
        .unwrap_or_default();

    let options = controller.options();

    QueryPageState {
        page_id: page.id.clone(),
        query: page.query.clone(),
        location: page.location.clone(),
        tag_service: page.tag_service.clone(),
        tag_services: query::local_tag_service_names(controller),
        sort: page.sort.clone(),
        sort_ascending: page.sort_ascending,
        focused,
        tags,
        predicates: page
            .query
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect(),
        media: query::media_items(controller, page),
        loading: page.loading,
        thumb_scale_percent: options.get_integer("thumbnail_scale_percent").clamp(25, 400) as i32,
        thumb_masonry: options.get_boolean("thumbnail_layout_masonry"),
        thumb_horizontal: options.get_boolean("thumbnail_scroll_horizontal"),
    }
}

/// Typed focus/selection update for `UiCommand::SetSelection` (WS-15.2.2b).
pub fn selection_state(
    controller: &crate::controller::Controller,
    page_id: &str,
    hash: &[u8],
) -> SelectionState {
    SelectionState {
        page_id: page_id.to_string(),
        hash: query::hex(hash),
        tags: query::selection_tags(controller, hash),
        preview_path: query::preview_path(controller, hash)
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
    }
}

/// Typed metadata panel contents (WS-15.2.2b). Same source data as
/// `metadata::metadata_json` (which stays for the tag dialog), minus the
/// JSON shapes the bridge cannot carry: the display-tag sources map becomes
/// a flat vector of pairs and every rating row carries a numeric kind.
pub fn metadata_state(controller: &crate::controller::Controller, hash: &[u8]) -> MetadataState {
    let Some(hash_id) = query::hash_id_for(controller, hash) else {
        return MetadataState {
            hash: query::hex(hash),
            tag_services: Vec::new(),
            ratings: Vec::new(),
            notes: Vec::new(),
        };
    };

    let tag_services = super::metadata::tag_services(controller, hash_id)
        .into_iter()
        .map(|service| TagServiceState {
            key: service.key,
            name: service.name,
            tags: service.tags,
            display_tags: service.display_tags,
            sources: service
                .display_tag_sources
                .into_iter()
                .map(|(display, storage)| TagSource { display, storage })
                .collect(),
        })
        .collect();

    let ratings = super::metadata::rating_services(controller, hash_id)
        .into_iter()
        .map(|service| match service.kind.as_str() {
            "numerical" => RatingState {
                key: service.key,
                name: service.name,
                kind: RatingKind::Numerical,
                value_num: service.value.as_f64().unwrap_or(0.0),
                value_text: String::new(),
                value_int: 0,
            },
            "like" => RatingState {
                key: service.key,
                name: service.name,
                kind: RatingKind::Like,
                value_num: 0.0,
                value_text: service.value.as_str().unwrap_or_default().to_string(),
                value_int: 0,
            },
            _ => RatingState {
                key: service.key,
                name: service.name,
                kind: RatingKind::IncDec,
                value_num: 0.0,
                value_text: String::new(),
                value_int: service.value.as_i64().unwrap_or(0),
            },
        })
        .collect();

    let notes = crate::db::notes::get_notes(controller.db().db(), hash_id)
        .unwrap_or_default()
        .into_iter()
        .map(|(name, text)| NoteState { name, text })
        .collect();

    MetadataState {
        hash: query::hex(hash),
        tag_services,
        ratings,
        notes,
    }
}

pub fn viewer_state(
    controller: &crate::controller::Controller,
    viewer_id: &str,
    page: &QueryPage,
    hash: &[u8],
) -> ViewerState {
    let position = query::viewer_position(page, hash);
    let total = page.hashes.len();
    let path = query::preview_path(controller, hash)
        .map(|path| path.display().to_string())
        .unwrap_or_default();

    let info = match query::file_info_for(controller, hash) {
        Some(info) => {
            let resolution = match (info.width, info.height) {
                (Some(width), Some(height)) => format!("{width} x {height}"),
                _ => "unknown resolution".to_string(),
            };

            format!(
                "{resolution} - {} - {}",
                human_size(info.size),
                info.mime.as_str()
            )
        }
        None => "no metadata".to_string(),
    };

    let file_path = query::file_path(controller, hash)
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let url = query::urls_for(controller, hash)
        .into_iter()
        .next()
        .unwrap_or_default();
    let neighbour_path = |direction: &str| {
        query::neighbour(page, hash, direction)
            .and_then(|neighbour| query::preview_path(controller, &neighbour))
            .map(|path| path.display().to_string())
            .unwrap_or_default()
    };
    let previous_path = neighbour_path("prev");
    let next_path = neighbour_path("next");

    let options = controller.options();
    let always_on_top = options.get_boolean("media_viewer_window_always_on_top")
        || options.get_boolean("always_start_media_viewers_always_on_top");
    let frameless = options.get_boolean("always_start_media_viewers_frameless");

    ViewerState {
        viewer_id: viewer_id.to_string(),
        page_id: page.id.clone(),
        hash: query::hex(hash),
        title: format!("{} - {}/{}", page.title, position + 1, total),
        path,
        file_path,
        url,
        previous_path,
        next_path,
        info: format!("{info} - file {}/{}", position + 1, total),
        always_on_top,
        frameless,
        recenter: options.get_boolean("media_viewer_recenter_media_on_window_resize"),
        zoom_step_percent: options
            .get_integer("media_viewer_zoom_step_percent")
            .clamp(5, 200) as i32,
        metadata: metadata_state(controller, hash),
    }
}

pub fn downloader_json(
    controller: &crate::controller::Controller,
    page: &DownloaderPage,
) -> String {
    let formula_name = if page.formula.trim().is_empty() {
        controller
            .options()
            .get_string("favourite_simple_downloader_formula")
    } else {
        page.formula.clone()
    };

    let queue: Vec<Value> = page
        .queue
        .iter()
        .rev()
        .take(400)
        .rev()
        .map(|item| {
            json!({
                "url": item.url,
                "domain": crate::networking::download_pools::domain_of(&item.url),
                "status": item.status.as_str(),
                "note": item.note,
            })
        })
        .collect();
    let queue_done = page
        .queue
        .iter()
        .filter(|item| {
            matches!(
                item.status,
                crate::gui::app::model::DownloadItemStatus::Done
                    | crate::gui::app::model::DownloadItemStatus::Error
            )
        })
        .count();
    let gallery_total = page.gallery_seeds.len();
    let gallery_done = page
        .gallery_seeds
        .iter()
        .filter(|seed| seed.status == "success" || seed.status == "failed")
        .count();
    let gallery_success = page
        .gallery_seeds
        .iter()
        .filter(|seed| seed.status == "success")
        .count();
    let gallery_failed = page
        .gallery_seeds
        .iter()
        .filter(|seed| seed.status == "failed")
        .count();

    json!({
        "page_id": page.id,
        "kind": page.kind,
        "title": page.title,
        "input": page.input,
        "url": page.url,
        "query": page.query,
        "site": page.site,
        "import_options_summary": super::download::import_options_summary(
            &page.import_options,
            super::download::downloader_caller(page.kind),
        ),
        "sites": crate::parsing::gallery::GALLERY_PARSERS
            .iter()
            .map(|parser| {
                json!({
                    "id": parser.id.as_str(),
                    "title": parser.title,
                    "template": parser.search_template,
                })
            })
            .collect::<Vec<_>>(),
        "filter": page.filter,
        "subject": page.subject,
        "paused": page.paused,
        "running": page.running,
        "auto_watch": page.auto_watch,
        "check_period": page.check_period,
        "status": page.status,
        "imported": page.imported,
        "failed": page.failed,
        "result_count": page.results.len(),
        "file_limit": page.file_limit,
        "last_check": page.last_check,
        "formula_name": formula_name,
        "pending_jobs": if page.kind == DownloaderKind::Simple {
            page.queue
                .iter()
                .map(|item| item.url.clone())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        },
        "queue_count": page.queue.len(),
        "queue_done": queue_done,
        "gallery_seed_count": gallery_total,
        "gallery_seed_done": gallery_done,
        "gallery_seed_success": gallery_success,
        "gallery_seed_failed": gallery_failed,
        "queue": queue,
        "log": page.log.iter().rev().take(200).cloned().collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n"),
    })
    .to_string()
}

pub fn subscriptions_json(state: &AppState, page_id: &str) -> String {
    let now = hydrus_core::hydrus_time::get_now();
    let page = state.find_subscriptions(page_id);

    // Python's HydrusTime.TimestampToPrettyTimeDelta shape, without the
    // shared ISO preference: "n/a", "just now", "5 minutes ago", "in 2 hours".
    let pretty_delta = |timestamp: i64| -> String {
        if timestamp <= 0 {
            return "n/a".to_string();
        }

        let delta = (now - timestamp).abs();
        let past = timestamp <= now;
        let text = if delta < 60 {
            "just now".to_string()
        } else if delta < 3600 {
            let value = delta / 60.max(1);
            format!("{value} minute{}", if value == 1 { "" } else { "s" })
        } else if delta < 86_400 {
            let value = delta / 3600;
            format!("{value} hour{}", if value == 1 { "" } else { "s" })
        } else {
            let value = delta / 86_400;
            format!("{value} day{}", if value == 1 { "" } else { "s" })
        };

        if text == "just now" {
            text
        } else if past {
            format!("{text} ago")
        } else {
            format!("in {text}")
        }
    };

    let subscriptions: Vec<Value> = state
        .subscriptions
        .iter()
        .map(|subscription| {
            let next = subscription.last_check + subscription.period.max(60);
            let source = crate::parsing::gallery::GALLERY_PARSERS
                .iter()
                .find(|parser| parser.id.as_str() == subscription.site)
                .map(|parser| parser.title.to_string())
                .unwrap_or_else(|| {
                    if subscription.site.trim().is_empty() {
                        "no downloader set!".to_string()
                    } else {
                        subscription.site.clone()
                    }
                });
            let pretty_status = if subscription.url.trim().is_empty() {
                "no queries".to_string()
            } else {
                "1 working".to_string()
            };
            let processed = subscription.imported + subscription.failed;
            let items = if processed == 0 {
                String::new()
            } else if subscription.failed > 0 {
                format!("{processed} - {}F", subscription.failed)
            } else {
                processed.to_string()
            };
            let last_new_file_time = if subscription.imported > 0 && subscription.last_check > 0 {
                pretty_delta(subscription.last_check)
            } else {
                "n/a".to_string()
            };

            json!({
                "name": subscription.name,
                "url": subscription.url,
                "query": subscription.query,
                "site": subscription.site,
                "filter": subscription.filter,
                "period": subscription.period,
                "paused": subscription.paused,
                "running": subscription.running,
                "last_check": subscription.last_check,
                "next_check": next,
                "due_in": (next - now).max(0),
                "imported": subscription.imported,
                "failed": subscription.failed,
                "status": subscription.status,
                "result_count": subscription.results.len(),
                "import_options_summary": super::download::import_options_summary(
                    &subscription.import_options,
                    "subscription",
                ),
                "custom_import_options": if subscription.import_options.is_empty() {
                    String::new()
                } else {
                    super::download::import_options_summary(
                        &subscription.import_options,
                        "subscription",
                    )
                },
                "import_options": serde_json::to_value(&subscription.import_options)
                    .unwrap_or(Value::Null),
                // Columns of EditSubscriptionsPanel.
                "source": source,
                "pretty_status": pretty_status,
                "last_new_file_time": last_new_file_time,
                "last_checked": pretty_delta(subscription.last_check),
                "error": "",
                "items": items,
                "paused_display": if subscription.paused { "yes" } else { "" },
            })
        })
        .collect();

    json!({
        "page_id": page_id,
        "title": page.map(|page| page.title.as_str()).unwrap_or("manage subscriptions"),
        "status": page.map(|page| page.status.as_str()).unwrap_or(""),
        "subs_paused": state.controller.options().get_boolean("pause_subs_sync"),
        "sites": crate::parsing::gallery::GALLERY_PARSERS
            .iter()
            .map(|parser| {
                json!({
                    "id": parser.id.as_str(),
                    "title": parser.title,
                    "template": parser.search_template,
                })
            })
            .collect::<Vec<_>>(),
        "subscriptions": subscriptions,
    })
    .to_string()
}

pub fn duplicates_json(
    controller: &crate::controller::Controller,
    page: &DuplicatesPage,
) -> String {
    let pair = page.current().map(|pair| {
        json!({
            "distance": pair.distance,
            "a": duplicate_media_json(controller, &pair.a),
            "b": duplicate_media_json(controller, &pair.b),
        })
    });

    json!({
        "page_id": page.id,
        "title": page.title,
        "status": page.status,
        "index": if page.pairs.is_empty() { 0 } else { page.index + 1 },
        "total": page.pairs.len(),
        "searched": page.searched,
        "matched": page.matched,
        "search": {
            "query_1": page.search_context.query_1,
            "location_1": page.search_context.location_1,
            "query_2": page.search_context.query_2,
            "location_2": page.search_context.location_2,
            "dupe_search_type": page.search_context.dupe_search_type,
            "pixel_dupes": page.search_context.pixel_dupes,
            "max_distance": page.search_context.max_distance,
            "sort_type": page.search_context.sort_type,
            "sort_ascending": page.search_context.sort_ascending,
            "summary": page.search_context.summary(),
            "matched": page.matched,
        },
        "batch_size": controller
            .options()
            .get_integer("duplicate_filter_max_batch_size")
            .clamp(5, 1024),
        "pair": pair,
        "media": crate::gui::app::duplicates::pair_thumbnail_hashes(page)
            .iter()
            .map(|hash| duplicate_media_json(controller, hash))
            .collect::<Vec<_>>(),
        "media_empty_status": crate::gui::app::duplicates::duplicates_media_empty_status(),
        "media_count_status": crate::gui::app::duplicates::duplicates_media_count_status(page.pairs.len()),
    })
    .to_string()
}

pub(crate) fn duplicate_media_json(
    controller: &crate::controller::Controller,
    hash: &[u8],
) -> Value {
    let info = query::file_info_for(controller, hash);
    let info_text = info
        .as_ref()
        .map(|info| {
            let resolution = match (info.width, info.height) {
                (Some(width), Some(height)) => format!("{width} x {height}"),
                _ => "unknown resolution".to_string(),
            };

            format!(
                "{resolution} - {} - {}",
                human_size(info.size),
                info.mime.as_str()
            )
        })
        .unwrap_or_else(|| "no metadata".to_string());

    json!({
        "hash": query::hex(hash),
        "thumb": query::thumbnail_path(controller, hash)
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        "preview": query::preview_path(controller, hash)
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        "info": info_text,
        "tags": query::selection_tags(controller, hash),
        "urls": query::hash_id_for(controller, hash)
            .and_then(|hash_id| crate::db::url_map::get_urls(controller.db().db(), hash_id).ok())
            .unwrap_or_default(),
        "inbox": query::hash_id_for(controller, hash)
            .and_then(|hash_id| crate::db::files_inbox::is_inbox(controller.db().db(), hash_id).ok())
            .unwrap_or(false),
    })
}

pub fn human_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{size} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
