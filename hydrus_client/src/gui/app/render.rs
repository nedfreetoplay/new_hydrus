//! JSON rendering of application state for the Qt layer.

use serde_json::{Value, json};

use super::model::{AppState, PageNode, QueryPage};
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
                {"label": "undo", "action": "undo", "enabled": false}
            ]
        },
        {
            "title": "&pages",
            "items": [
                {"label": "new page...", "action": "new_page"},
                {"label": "refresh", "action": "refresh_page"},
                {"separator": true},
                {"label": "new page of pages", "action": "new_notebook"},
                {"label": "new duplicates processing page", "action": "new_duplicates"},
                {"separator": true},
                {"label": "close page", "action": "close_page"},
                {"label": "close other pages", "action": "close_other_pages"}
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
            "title": "&network",
            "items": [
                {"label": "manage network...", "action": "options_network"},
                {"label": "pause all new network traffic", "action": "pause_network", "checkable": true, "checked": false}
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

    json!({
        "query": page.query,
        "status": page.status,
        "empty_text": page.empty_text,
        "focused": focused,
        "tags": tags,
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

            format!("{position}{}", page.status)
        }
        None => String::new(),
    };

    let version = state.controller.db().get_version().unwrap_or(0);
    let services = state.controller.services().get_num_services();
    let dir = state.controller.db_dir().display();

    json!({
        "message": state.message,
        "media": media,
        "idle": "idle",
        "db": format!("db {version} - {services} services - {dir}"),
    })
    .to_string()
}

pub fn viewer_json(
    controller: &crate::controller::Controller,
    viewer_id: &str,
    page: &QueryPage,
    hash: &[u8],
) -> String {
    let position = query::viewer_position(page, hash);
    let total = page.hashes.len();
    let path = query::file_path(controller, hash)
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

    json!({
        "viewer_id": viewer_id,
        "page_id": page.id,
        "hash": query::hex(hash),
        "title": format!("{} - {}/{}", page.title, position + 1, total),
        "path": path,
        "tags": query::selection_tags(controller, hash),
        "info": format!("{info} - file {}/{}", position + 1, total),
    })
    .to_string()
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
