//! JSON rendering of application state for the Qt layer.

use serde_json::{Value, json};

use hydrus_core::hydrus_constants::ServiceType;

use super::model::{
    AppState, DownloaderKind, DownloaderPage, DuplicatesPage, PageNode, QueryPage,
};
use super::query;

pub fn menus_json(state: &AppState) -> String {
    let services = state.controller.services();
    let mut service_entries: Vec<Value> = services
        .get_services(None)
        .into_iter()
        .map(|service| {
            json!({
                "label": service.name,
                "action": format!("service_info:{}", service.name),
                "description": "show information about this service",
            })
        })
        .collect();

    let tag_filter_entries: Vec<Value> = services
        .get_services(Some(&[ServiceType::TagRepository, ServiceType::LocalTag]))
        .into_iter()
        .map(|service| {
            json!({
                "label": service.name,
                "action": format!("tag_filter:{}", query::hex(&service.service_key)),
                "description": "edit the tag filter and purge tags",
            })
        })
        .collect();

    if !tag_filter_entries.is_empty() {
        service_entries.push(json!({"separator": true}));
        service_entries.push(json!({
            "label": "tag filters",
            "items": tag_filter_entries,
        }));
    }

    service_entries.push(json!({"separator": true}));
    service_entries.push(json!({
        "label": "manage services...",
        "action": "manage_services",
        "description": "local services are managed here"
    }));
    service_entries.push(json!({
        "label": "client api permissions...",
        "action": "api_permissions",
        "description": "manage Client API access keys and permissions"
    }));

    let mut menus = json!([
        {
            "title": "&file",
            "items": [
                {"label": "import files...", "action": "import_files", "description": "Add new files to the database."},
                {"label": "import options...", "action": "import_options", "description": "Choose file filters, destinations, tags, notes and presentation for imports."},
                {"label": "import url...", "action": "import_url", "description": "Download one file from a URL and import it."},
                {"label": "manage import folders\u{2026}", "action": "manage_import_folders", "description": "Manage folders from which the client can automatically import."},
                {"label": "import folders", "items": import_folder_menu_items(state)},
                {"label": "export folders", "items": export_folder_menu_items(state)},
                {"separator": true},
                {"label": "open", "items": [
                    {"label": "installation directory", "action": "open_install_dir"},
                    {"label": "database directory", "action": "open_db_dir"}
                ]},
                {"separator": true},
                {"label": "options...", "action": "options"},
                {"separator": true},
                {"label": "exit", "action": "exit"}
            ]
        },
        {
            "title": "&undo",
            "items": [
                {"label": "undo", "action": "undo", "enabled": !state.closed_pages.is_empty(),
                 "description": "Restore the most recently closed page."}
            ]
        },
        {
            "title": "&pages",
            "items": [
                {"label": "new page...", "action": "new_page"},
                {"label": "download", "items": [
                    {"label": "new url download page", "action": "new_urls", "description": "Download direct file URLs."},
                    {"label": "new watcher page", "action": "new_watcher", "description": "Watch a page for new files."},
                    {"label": "new gallery page", "action": "new_gallery", "description": "Parse a gallery page and download its files."},
                    {"label": "new simple downloader page", "action": "new_simple", "description": "Parse page links and download the files they point to."}
                ]},
                {"label": "refresh", "action": "refresh_page"},
                {"separator": true},
                {"label": "back", "action": "history_back", "enabled": state.history.can_back()},
                {"label": "forward", "action": "history_forward", "enabled": state.history.can_forward()},
                {"separator": true},
                {"label": "undo search", "action": "undo_search",
                 "enabled": state.active_query().is_some_and(|page| !page.query_undo.is_empty())},
                {"label": "redo search", "action": "redo_search",
                 "enabled": state.active_query().is_some_and(|page| !page.query_redo.is_empty())},
                {"label": "recent searches", "items": recent_search_items(state)},
                {"separator": true},
                {"label": "new page of pages", "action": "new_notebook"},
                {"label": "new duplicates processing page", "action": "new_duplicates"},
                {"separator": true},
                {"label": "manage pages...", "action": "page_manager"},
                {"label": "duplicate page", "action": "duplicate_page"},
                {"separator": true},
                {"label": "close page", "action": "close_page"},
                {"label": "close other pages", "action": "close_other_pages"},
                {"separator": true},
                {"label": "favourite searches", "items": favourite_menu_items(state)},
                {"separator": true},
                {"label": "sessions", "items": session_menu_items(state)}
            ]
        },
        {
            "title": "&database",
            "items": [
                {"label": "database directory", "action": "open_db_dir"},
                {"label": "show database info", "action": "db_info"},
                {"label": "back up database...", "action": "db_backup_open",
                 "description": "Copy client.db to the configured backup directory."},
                {"separator": true},
                {"label": "maintenance", "items": [
                    {"label": "check database", "action": "db_check"},
                    {"label": "analyze database", "action": "db_analyze"},
                    {"label": "vacuum database", "action": "db_vacuum"},
                    {"label": "review vacuum data", "action": "db_vacuum_info"},
                    {"label": "review deferred deletes", "action": "db_deferred_deletes"},
                    {"label": "fix missing archive times", "action": "db_fix_archive_times"},
                    {"separator": true},
                    {"label": "review file maintenance jobs", "action": "db_file_maintenance"},
                    {"label": "run file maintenance jobs", "action": "db_run_file_maintenance"},
                    {"separator": true},
                    {"label": "clear orphan files...", "action": "orphan_files_scan"},
                    {"separator": true},
                    {"label": "review local imports...", "action": "import_files"},
                    {"label": "repair file locations...", "action": "repair_file_locations"},
                    {"separator": true},
                    {"label": "how boned am I?", "action": "db_stats"}
                ]}
            ]
        },
        {
            "title": "&network",
            "items": [
                {"label": "manage network...", "action": "options_network"},
                {"label": "pause all new network traffic", "action": "pause_network", "checkable": true, "checked": state.controller.network_paused()},
                {"label": "subscriptions\u{2026}", "action": "manage_subscriptions", "description": "Change the queries you want the client to regularly import from."},
                {"separator": true},
                {"label": "export downloader definition...", "action": "downloader_export"},
                {"label": "import downloader definition...", "action": "downloader_import"}
            ]
        },
        {
            "title": "&services",
            "items": service_entries
        },
        {
            "title": "&tags",
            "items": [
                {"label": "manage tags...", "action": "manage_tags_active"},
                {"label": "manage notes...", "action": "manage_notes_active"},
                {"label": "edit timestamps...", "action": "edit_timestamps"},
                {"separator": true},
                {"label": "copy tags", "action": "copy_tags"},
                {"label": "paste tags...", "action": "paste_tags_prompt"},
                {"label": "apply tags to selection...", "action": "apply_tags_selection"},
                {"separator": true},
                {"label": "duplicate auto-resolution...", "action": "duplicates_rules"},
                {"label": "review duplicate pending decisions...", "action": "duplicates_review"},
                {"separator": true},
                {"label": "manage tag siblings...", "action": "tag_siblings"},
                {"label": "manage tag parents...", "action": "tag_parents"}
            ]
        },
        {
            "title": "&pending",
            "items": [
                {"label": "pending uploads", "action": "pending", "enabled": false}
            ]
        },
        {
            "title": "&help",
            "items": [
                {"label": "documentation", "action": "docs"},
                {"label": "popup messages...", "action": "notifications"},
                {"label": "command palette...", "action": "command_palette", "description": "Ctrl+K"},
                {"label": "about", "action": "about"},
                {"separator": true},
                {"label": "debug", "items": [
                    {"label": "show database info", "action": "db_info"}
                ]}
            ]
        }
    ]);

    // Python hides the pending menu unless a repository service exists
    // (`has_pending_services` in ClientGUI).
    let has_pending_services = !services
        .get_services(Some(&[
            ServiceType::TagRepository,
            ServiceType::FileRepository,
            ServiceType::Ipfs,
        ]))
        .is_empty();

    if !has_pending_services {
        if let Some(items) = menus.as_array_mut() {
            items.retain(|menu| {
                menu.get("title").and_then(Value::as_str) != Some("&pending")
            });
        }
    }

    menus.to_string()
}

fn import_folder_menu_items(state: &AppState) -> Vec<Value> {
    let mut items = vec![
        json!({"label": "check all", "action": "import_folder_check_all"}),
        json!({"separator": true}),
    ];

    for folder in &state.import_folders {
        items.push(json!({
            "label": folder.name,
            "action": "import_folder_check",
            "payload": folder.name,
        }));
    }

    if state.import_folders.is_empty() {
        items.push(json!({"label": "no import folders", "action": "", "enabled": false}));
    }

    items
}

fn export_folder_menu_items(state: &AppState) -> Vec<Value> {
    let mut items = vec![
        json!({"label": "manage export folders...", "action": "new_export_folders"}),
        json!({
            "label": "run all export folders",
            "action": "export_folder_run_all",
            "enabled": !state.export_folders.is_empty(),
        }),
        json!({"separator": true}),
    ];

    for folder in &state.export_folders {
        items.push(json!({
            "label": folder.name,
            "action": "export_folder_run",
            "payload": folder.name,
            "enabled": !folder.running && !folder.paused,
            "description": folder.status,
        }));
    }

    if state.export_folders.is_empty() {
        items.push(json!({"label": "no export folders", "action": "", "enabled": false}));
    }

    items
}

fn recent_search_items(state: &AppState) -> Vec<Value> {
    let mut items = Vec::new();

    for query in state.search_history.iter().take(15) {
        items.push(json!({
            "label": query,
            "action": "apply_search_history",
            "target": query,
        }));
    }

    if items.is_empty() {
        items.push(json!({"label": "no recent searches", "action": "", "enabled": false}));
    }

    items
}

fn favourite_menu_items(state: &AppState) -> Vec<Value> {
    let mut items = Vec::new();

    for favourite in &state.favourites {
        items.push(json!({
            "label": format!("{} — {}", favourite.name, favourite.query),
            "action": "favourite_apply",
            "target": favourite.name,
        }));
    }

    if items.is_empty() {
        items.push(json!({"label": "no favourite searches", "action": "", "enabled": false}));
    }

    items.push(json!({"separator": true}));
    items.push(json!({
        "label": "save current search...",
        "action": "favourite_save",
        "description": "Store the current query, file domain and tag service."
    }));

    let mut delete = Vec::new();

    for favourite in &state.favourites {
        delete.push(json!({
            "label": favourite.name,
            "action": "favourite_delete",
            "target": favourite.name,
        }));
    }

    if delete.is_empty() {
        delete.push(json!({"label": "no favourite searches", "action": "", "enabled": false}));
    }

    items.push(json!({"label": "delete", "items": delete}));

    items
}

fn session_menu_items(state: &AppState) -> Vec<Value> {
    let mut items = vec![
        json!({"label": "save current as...", "action": "session_save_as",
               "description": "Save the open pages under a new session name."}),
        json!({"label": "backup current", "action": "session_backup",
               "description": "Copy the open pages into the backup folder."}),
        json!({"separator": true}),
    ];

    let mut load = Vec::new();
    let mut append = Vec::new();
    let mut delete = Vec::new();

    for session in super::sessions::list(state) {
        let label = format!("{} ({} pages)", session.name, session.pages);
        load.push(json!({"label": label, "action": "session_load", "target": session.name}));
        append.push(json!({"label": label, "action": "session_append", "target": session.name}));
        delete.push(json!({"label": label, "action": "session_delete", "target": session.name}));
    }

    if load.is_empty() {
        load.push(json!({"label": "no saved sessions", "action": "", "enabled": false}));
        append.push(json!({"label": "no saved sessions", "action": "", "enabled": false}));
        delete.push(json!({"label": "no saved sessions", "action": "", "enabled": false}));
    }

    items.push(json!({"label": "load", "items": load}));
    items.push(json!({"label": "append", "items": append}));

    let backups = super::sessions::list_backups(state);
    let mut append_backup = Vec::new();

    for backup in backups {
        append_backup.push(json!({
            "label": format!("{} ({} pages)", backup.name, backup.pages),
            "action": "session_append_backup",
            "target": backup.name
        }));
    }

    if append_backup.is_empty() {
        append_backup.push(json!({"label": "no backups", "action": "", "enabled": false}));
    }

    items.push(json!({"label": "append backup", "items": append_backup}));
    items.push(json!({"label": "delete", "items": delete}));

    items
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

pub fn page_manager_json(state: &AppState) -> String {
    json!({"root": manager_node(state, &state.root)}).to_string()
}
fn manager_node(state: &AppState, notebook: &super::model::Notebook) -> Value {
    let children: Vec<Value> = notebook
        .children
        .iter()
        .map(|child| manager_item(state, child))
        .collect();

    json!({
        "id": notebook.id,
        "kind": "notebook",
        "title": notebook.title,
        "count": children.len(),
        "active": state.active.as_deref() == Some(notebook.id.as_str()),
        "children": children,
    })
}

fn manager_item(state: &AppState, node: &PageNode) -> Value {
    match node {
        PageNode::Notebook(notebook) => manager_node(state, notebook),
        PageNode::Query(page) => json!({
            "id": page.id,
            "kind": "query",
            "title": page.title,
            "count": page.hashes.len(),
            "active": state.active.as_deref() == Some(page.id.as_str()),
            "children": [],
        }),
        PageNode::Downloader(page) => json!({
            "id": page.id,
            "kind": "downloader",
            "title": page.title,
            "count": page.results.len(),
            "active": state.active.as_deref() == Some(page.id.as_str()),
            "children": [],
        }),
        PageNode::Subscriptions(page) => json!({
            "id": page.id,
            "kind": "subscriptions",
            "title": page.title,
            "count": state.subscriptions.len(),
            "active": state.active.as_deref() == Some(page.id.as_str()),
            "children": [],
        }),
        PageNode::Duplicates(page) => json!({
            "id": page.id,
            "kind": "duplicates",
            "title": page.title,
            "count": page.pairs.len(),
            "active": state.active.as_deref() == Some(page.id.as_str()),
            "children": [],
        }),
        PageNode::ImportFolders(page) => json!({
            "id": page.id,
            "kind": "import_folders",
            "title": page.title,
            "count": state.import_folders.len(),
            "active": state.active.as_deref() == Some(page.id.as_str()),
            "children": [],
        }),
        PageNode::ExportFolders(page) => json!({
            "id": page.id,
            "kind": "export_folders",
            "title": page.title,
            "count": state.export_folders.len(),
            "active": state.active.as_deref() == Some(page.id.as_str()),
            "children": [],
        }),
        PageNode::Placeholder(page) => json!({
            "id": page.id,
            "kind": "placeholder",
            "title": page.title,
            "count": 0,
            "active": state.active.as_deref() == Some(page.id.as_str()),
            "children": [],
        }),
    }
}

pub fn pages_json(state: &AppState) -> String {
    json!({"root": notebook_node(state, &state.root)}).to_string()
}

fn notebook_node(state: &AppState, notebook: &super::model::Notebook) -> Value {
    let children: Vec<Value> = notebook
        .children
        .iter()
        .map(|child| node_json(state, child))
        .collect();

    json!({
        "id": notebook.id,
        "kind": "notebook",
        "title": notebook.title,
        "current": notebook.current,
        "children": children,
    })
}

fn node_json(state: &AppState, node: &PageNode) -> Value {
    match node {
        PageNode::Query(page) => json!({
            "id": page.id,
            "kind": "query",
            "title": page.title,
            "state": query_state(&state.controller, page, Some(&state.import_queue)),
        }),
        PageNode::Notebook(notebook) => notebook_node(state, notebook),
        PageNode::Downloader(page) => json!({
            "id": page.id,
            "kind": "downloader",
            "title": page.title,
            "state": serde_json::from_str::<Value>(&downloader_json(&state.controller, page))
                .unwrap_or(Value::Null),
        }),
        PageNode::Subscriptions(page) => json!({
            "id": page.id,
            "kind": "subscriptions",
            "title": page.title,
            "state": serde_json::from_str::<Value>(&subscriptions_json(state, &page.id))
                .unwrap_or(Value::Null),
        }),
        PageNode::Duplicates(page) => json!({
            "id": page.id,
            "kind": "duplicates",
            "title": page.title,
            "state": serde_json::from_str::<Value>(&duplicates_json(&state.controller, page))
                .unwrap_or(Value::Null),
        }),
        PageNode::ImportFolders(page) => json!({
            "id": page.id,
            "kind": "import_folders",
            "title": page.title,
            "state": serde_json::from_str::<Value>(&super::import_folders::rules_json(state, &page.id))
                .unwrap_or(Value::Null),
        }),
        PageNode::ExportFolders(page) => json!({
            "id": page.id,
            "kind": "export_folders",
            "title": page.title,
            "state": serde_json::from_str::<Value>(&super::export::folders_json(state, &page.id))
                .unwrap_or(Value::Null),
        }),
        PageNode::Placeholder(page) => json!({
            "id": page.id,
            "kind": "placeholder",
            "title": page.title,
            "state": {"message": page.message},
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
pub fn query_progress_json(
    page: &QueryPage,
    import: Option<&super::model::ImportQueueState>,
) -> String {
    json!({
        "page_id": page.id,
        "loading": page.loading,
        "importing": importing_json(import),
        "count": page.hashes.len(),
    })
    .to_string()
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

pub fn about_json(state: &AppState) -> String {
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

    json!({
        "title": "about",
        "software_version": hydrus_core::hydrus_constants::SOFTWARE_VERSION,
        "network_version": hydrus_core::hydrus_constants::NETWORK_VERSION,
        "client_api_version": hydrus_core::hydrus_constants::CLIENT_API_VERSION,
        "db_version": controller.db().get_version().unwrap_or(0),
        "db_dir": controller.db_dir().display().to_string(),
        "install_dir": install_dir,
        "services": controller.services().get_num_services(),
        "locations": controller.files_manager().locations().len(),
        "shortcuts": controller.shortcuts().len(),
        "api": controller.api_status().unwrap_or_else(|| "stopped".to_string()),
        "sqlite": sqlite,
        "rust": env!("CARGO_PKG_VERSION"),
        "license": "Hydrus is free software, released under the WTFPL.",
        "site": "https://hydrusnetwork.github.io/hydrus/",
        "credits": "Hydrus Network by the Hydrus developer and contributors.",
    })
    .to_string()
}

pub fn status_json(state: &AppState) -> String {
    let media = match state.active_query() {
        Some(page) => {
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
        None => match state.active_downloader() {
            Some(page)
                if page.kind == crate::gui::app::model::DownloaderKind::Watcher
                    && !page.url.is_empty() =>
            {
                "no files for this watcher and its publishing settings".to_string()
            }
            Some(page) if page.kind == crate::gui::app::model::DownloaderKind::Watcher => {
                "no highlighted watcher".to_string()
            }
            Some(page) if page.kind == crate::gui::app::model::DownloaderKind::Gallery => {
                "no highlighted query".to_string()
            }
            Some(page) if page.queue.is_empty() && page.results.is_empty() => {
                "empty page".to_string()
            }
            Some(page) => format!("{} files", page.results.len()),
            None => String::new(),
        },
    };

    let mut busy = String::new();

    if state.import_queue.running {
        busy = format!("{}/{}", state.import_queue.done, state.import_queue.total);
    } else if let Some((title, done, total)) = running_downloader(state) {
        busy = format!("{title}: {done}/{total}");
    }

    let idle = if busy.is_empty() {
        "idle".to_string()
    } else {
        String::new()
    };

    let bandwidth = if state.controller.network_paused() {
        "network paused".to_string()
    } else {
        "0B".to_string()
    };

    json!({
        "message": state.message,
        "media": media,
        "bandwidth": bandwidth,
        "idle": idle,
        "hydrus_busy": if busy.is_empty() { String::new() } else { "working".to_string() },
        "cpu_busy": "",
        "db": "",
    })
    .to_string()
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

pub fn viewing_stats_json(controller: &crate::controller::Controller, hash: &[u8]) -> String {
    let Some(hash_id) = query::hash_id_for(controller, hash) else {
        return json!({"hash": query::hex(hash), "rows": []}).to_string();
    };

    let db = controller.db().db();
    let mut rows = Vec::new();

    for (canvas_type, name) in [(0i64, "media"), (1i64, "preview")] {
        let stats = crate::db::files_viewing_stats::get_viewing_stats(db, hash_id, canvas_type)
            .unwrap_or_default();

        rows.push(json!({
            "canvas": name,
            "last_viewed": hydrus_core::hydrus_time::timestamp_to_pretty_time(
                stats.last_viewed_timestamp_ms.map(|value| value as f64 / 1000.0),
                false,
                true,
            ),
            "views": stats.views,
            "viewtime": stats.viewtime_ms / 1000,
        }));
    }

    json!({"hash": query::hex(hash), "rows": rows}).to_string()
}

#[cfg(test)]
mod tests;

pub fn viewer_json(
    controller: &crate::controller::Controller,
    viewer_id: &str,
    page: &QueryPage,
    hash: &[u8],
) -> String {
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

    json!({
        "viewer_id": viewer_id,
        "page_id": page.id,
        "hash": query::hex(hash),
        "title": format!("{} - {}/{}", page.title, position + 1, total),
        "path": path,
        "file_path": file_path,
        "url": url,
        "previous_path": previous_path,
        "next_path": next_path,
        "tags": query::selection_tags(controller, hash),
        "metadata": super::metadata::metadata_json(controller, hash),
        "info": format!("{info} - file {}/{}", position + 1, total),
        "always_on_top": always_on_top,
        "frameless": frameless,
        "muted": options.get_boolean("media_viewer_audio_mute"),
        "recenter": options.get_boolean("media_viewer_recenter_media_on_window_resize"),
        "zoom_step_percent": options.get_integer("media_viewer_zoom_step_percent").clamp(5, 200),
        "prefetch_previous": options.get_integer("media_viewer_prefetch_num_previous").clamp(0, 25),
        "prefetch_next": options.get_integer("media_viewer_prefetch_num_next").clamp(0, 25),
    })
    .to_string()
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
