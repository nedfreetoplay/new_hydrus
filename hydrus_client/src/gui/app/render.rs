//! JSON rendering of application state for the Qt layer.

use serde_json::{Value, json};

use super::model::{AppState, DownloaderPage, DuplicatesPage, PageNode, QueryPage};
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

    service_entries.push(json!({"separator": true}));
    service_entries.push(json!({
        "label": "manage services...",
        "action": "manage_services",
        "description": "local services are managed here"
    }));

    let menus = json!([
        {
            "title": "&file",
            "items": [
                {"label": "import files...", "action": "import_files", "description": "Add new files to the database."},
                {"label": "import url...", "action": "import_url", "description": "Download one file from a URL and import it."},
                {"label": "import folders...", "action": "new_import_folders", "description": "Scan folders for new files."},
                {"label": "import folders", "items": import_folder_menu_items(state)},
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
                {"separator": true},
                {"label": "maintenance", "items": [
                    {"label": "check database", "action": "db_check"},
                    {"label": "vacuum database", "action": "db_vacuum"}
                ]}
            ]
        },
        {
            "title": "&download",
            "items": [
                {"label": "urls", "action": "new_urls", "description": "Download direct file URLs."},
                {"label": "gallery", "action": "new_gallery", "description": "Parse a gallery page and download its files."},
                {"label": "watcher", "action": "new_watcher", "description": "Watch a page for new files."},
                {"label": "subscriptions", "action": "new_subscriptions", "description": "Manage periodic gallery subscriptions."},
                {"label": "simple downloader", "action": "new_simple", "description": "Parse page links and download the files they point to."}
            ]
        },
        {
            "title": "&network",
            "items": [
                {"label": "manage network...", "action": "options_network"},
                {"label": "pause all new network traffic", "action": "pause_network", "checkable": true, "checked": state.controller.network_paused()}
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
                {"separator": true},
                {"label": "duplicate auto-resolution...", "action": "duplicates_rules"},
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
                {"label": "about", "action": "about"},
                {"separator": true},
                {"label": "debug", "items": [
                    {"label": "show database info", "action": "db_info"}
                ]}
            ]
        }
    ]);

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
            "state": query_state(&state.controller, page),
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
        PageNode::Placeholder(page) => json!({
            "id": page.id,
            "kind": "placeholder",
            "title": page.title,
            "state": {"message": page.message},
        }),
    }
}

pub fn query_state(controller: &crate::controller::Controller, page: &QueryPage) -> Value {
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
        "metadata": metadata,
        "media": query::media_json(controller, page),
    })
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
        None => String::new(),
    };

    let version = state.controller.db().get_version().unwrap_or(0);
    let services = state.controller.services().get_num_services();
    let dir = state.controller.db_dir().display();

    let mut idle = "idle".to_string();

    if state.import_queue.running {
        idle = format!(
            "importing {}/{}",
            state.import_queue.done, state.import_queue.total
        );
    } else if let Some((title, done, total)) = running_downloader(state) {
        idle = format!("{title}: {done}/{total}");
    }

    if state.controller.network_paused() {
        idle.push_str(" - network paused");
    }

    if let Some(address) = state.controller.api_status() {
        idle.push_str(&format!(" - api {address}"));
    }

    json!({
        "message": state.message,
        "media": media,
        "idle": idle,
        "db": format!("db {version} - {services} services - {dir}"),
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
    let _ = controller;

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

    json!({
        "page_id": page.id,
        "kind": page.kind,
        "title": page.title,
        "input": page.input,
        "url": page.url,
        "query": page.query,
        "site": page.site,
        "sites": crate::parsing::gallery::GALLERY_PARSERS
            .iter()
            .map(|parser| json!({"id": parser.id.as_str(), "title": parser.title}))
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
        "queue": queue,
        "log": page.log.iter().rev().take(200).cloned().collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n"),
    })
    .to_string()
}

pub fn subscriptions_json(state: &AppState, page_id: &str) -> String {
    let now = hydrus_core::hydrus_time::get_now();
    let page = state.find_subscriptions(page_id);

    let subscriptions: Vec<Value> = state
        .subscriptions
        .iter()
        .map(|subscription| {
            let next = subscription.last_check + subscription.period.max(60);
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
            })
        })
        .collect();

    json!({
        "page_id": page_id,
        "title": page.map(|page| page.title.as_str()).unwrap_or("subscriptions"),
        "status": page.map(|page| page.status.as_str()).unwrap_or(""),
        "sites": crate::parsing::gallery::GALLERY_PARSERS
            .iter()
            .map(|parser| json!({"id": parser.id.as_str(), "title": parser.title}))
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
        "pair": pair,
    })
    .to_string()
}

fn duplicate_media_json(controller: &crate::controller::Controller, hash: &[u8]) -> Value {
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
