//! Application state and event handling for the real client GUI.

use std::cell::RefCell;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use hydrus_core::hydrus_constants::{ContentStatus, SOFTWARE_VERSION, dirs};
use hydrus_core::hydrus_time;
use serde_json::{Value, json};

use crate::controller::Controller;
use crate::db::{files_storage, mappings, master_tags};
use crate::gui::qt::bridge::ffi;
use crate::gui::qt::panels;

pub mod commands;
pub mod import;
pub mod model;
pub mod query;
pub mod render;
pub mod session;

pub use commands::UiCommand;
use model::{AppState, Notebook, PageNode, PlaceholderPage, QueryPage, ViewerContext};

thread_local! {
    static STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

pub fn install(controller: Arc<Controller>) {
    let state = AppState::new(controller);
    STATE.with(|slot| *slot.borrow_mut() = Some(state));

    with_state_mut(|state| state.refresh_all());
}

pub fn initial_commands() -> Vec<UiCommand> {
    with_state_mut(|state| {
        state.refresh_all();

        let tags = serde_json::to_string(&query::all_tags(&state.controller))
            .unwrap_or_else(|_| "[]".to_string());

        vec![
            UiCommand::Menus(render::menus_json(state)),
            UiCommand::TagSuggestions(tags),
            UiCommand::BuildPages(render::pages_json(state)),
            UiCommand::Status(render::status_json(state)),
        ]
    })
}

pub fn save_session() {
    let _ = with_state(|state| session::save(state));
}

/// Development helper: imports files before the first render. Used by the
/// screenshot smoke test and useful for manual checks.
pub fn import_for_testing(paths: &[String]) {
    with_state_mut(|state| {
        let paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        let summary = import::import_paths(&state.controller, &paths);
        state.message = summary.message();
        state.refresh_all();
    });
}

/// Development helper: applies a tag to the first file on the active page.
pub fn add_tag_for_testing(tag: &str) {
    with_state_mut(|state| {
        let Some(page) = state.active_query() else {
            return;
        };
        let Some(hash) = page.hashes.first().cloned() else {
            return;
        };

        let Some(hash_id) = query::hash_id_for(&state.controller, &hash) else {
            return;
        };
        let Some(service_id) = query::tag_service_id(&state.controller) else {
            return;
        };

        if let Ok(tag_id) = master_tags::get_tag_id(state.controller.db().db(), tag) {
            let _ = mappings::add_mappings(
                state.controller.db().db(),
                service_id,
                ContentStatus::Current,
                hash_id,
                &[tag_id],
            );
            state.message = format!("added tag '{tag}'");
        }
    });
}

pub fn set_query_for_testing(query: &str) {
    with_state_mut(|state| {
        if let Some(page) = state.active_query() {
            let id = page.id.clone();

            if let Some(page) = state.find_query_mut(&id) {
                page.query = query.to_string();
            }
        }
    });
}

/// Development helper: runs `action|target|payload` steps separated by `;`.
pub fn run_script_for_testing(script: &str) -> Vec<UiCommand> {
    with_state_mut(|state| {
        let mut commands = Vec::new();

        for step in script.split(';') {
            let mut parts = step.split('|');
            let action = parts.next().unwrap_or_default().trim();
            let target = parts.next().unwrap_or_default().trim();
            let payload = parts.next().unwrap_or_default().trim();

            if !action.is_empty() {
                commands.extend(state.dispatch(action, target, payload));
            }
        }

        commands
    })
}

pub fn focus_first_for_testing() -> Vec<UiCommand> {
    with_state_mut(|state| {
        let Some(page) = state.active_query() else {
            return vec![];
        };
        let (page_id, hash) = (page.id.clone(), page.hashes.first().cloned());

        match hash {
            Some(hash) => state.focus(&page_id, &query::hex(&hash)),
            None => vec![],
        }
    })
}

pub fn open_first_for_testing() -> Vec<UiCommand> {
    with_state_mut(|state| {
        let Some(page) = state.active_query() else {
            return vec![];
        };
        let (page_id, hash) = (page.id.clone(), page.hashes.first().cloned());

        match hash {
            Some(hash) => state.open_viewer(&page_id, &query::hex(&hash)),
            None => vec![],
        }
    })
}

pub fn handle_action(
    mut window: Pin<&mut ffi::HydrusMainWindow>,
    action: &str,
    target: &str,
    payload: &str,
) {
    let commands = with_state_mut(|state| state.dispatch(action, target, payload));

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

fn with_state_mut<R>(call: impl FnOnce(&mut AppState) -> R) -> R {
    STATE.with(|slot| {
        let mut borrow = slot.borrow_mut();
        let state = borrow
            .as_mut()
            .expect("GUI application state is not installed");

        call(state)
    })
}

fn with_state<R>(call: impl FnOnce(&AppState) -> R) -> R {
    STATE.with(|slot| {
        let borrow = slot.borrow();
        let state = borrow
            .as_ref()
            .expect("GUI application state is not installed");

        call(state)
    })
}

impl AppState {
    fn status_command(&self) -> UiCommand {
        UiCommand::Status(render::status_json(self))
    }

    fn refresh_all(&mut self) {
        let controller = self.controller.clone();
        refresh_notebook(&mut self.root, &controller);
    }

    fn query_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        collect_query_ids(&self.root, &mut ids);
        ids
    }

    fn dispatch(&mut self, action: &str, target: &str, payload: &str) -> Vec<UiCommand> {
        match action {
            "page_selected" => {
                self.active = Some(target.to_string());
                vec![self.status_command()]
            }
            "new_page_menu" => {
                let notebook = if target.is_empty() {
                    self.active_notebook_id()
                } else {
                    target.to_string()
                };

                vec![UiCommand::ShowPageChooser(notebook)]
            }
            "new_page" => vec![UiCommand::ShowPageChooser(self.active_notebook_id())],
            "new_notebook" => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "notebook")
            }
            "new_duplicates" => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "duplicates")
            }
            "create_page" => self.create_page(target, payload),
            "close_page" => self.close_page(target),
            "close_other_pages" => self.close_other_pages(target),
            "rename_page" => self.rename_page(target, payload),
            "refresh_page" => {
                let id = self.page_id_or_active(target);
                self.refresh_page_commands(&id)
            }
            "search" => {
                let controller = self.controller.clone();

                if let Some(page) = self.find_query_mut(target) {
                    page.query = payload.to_string();
                }

                self.replace_query_and_refresh(&controller, target)
            }
            "focus_hash" => self.focus(target, payload),
            "open_viewer" => self.open_viewer(target, payload),
            "viewer_nav" => self.viewer_nav(target, payload),
            "viewer_close" => {
                self.viewers.remove(target);
                vec![]
            }
            "manage_tags" => self.manage_tags(target, payload, false),
            "manage_tags_active" => self.manage_tags("", "", true),
            "apply_tags" => self.apply_tags(payload),
            "archive_files" => self.set_inbox(target, payload, false),
            "inbox_files" => self.set_inbox(target, payload, true),
            "archive_active" => self.set_inbox_active(false),
            "inbox_active" => self.set_inbox_active(true),
            "delete_files" => self.delete_files(target, payload),
            "import_files" => vec![UiCommand::PickFiles],
            "import_url" => vec![UiCommand::PromptUrl],
            "download_url" => {
                let summary = import::import_url(&self.controller, payload);
                self.message = summary.message();
                self.refresh_commands()
            }
            "import_paths" => self.import_paths(payload),
            "exit" => vec![UiCommand::Quit],
            "open_db_dir" => vec![UiCommand::OpenPath(
                self.controller.db_dir().display().to_string(),
            )],
            "open_install_dir" => vec![UiCommand::OpenPath(dirs().base_dir.display().to_string())],
            "docs" => vec![UiCommand::OpenUrl(
                "https://hydrusnetwork.github.io/hydrus/".to_string(),
            )],
            "about" => vec![UiCommand::Message(
                "about hydrus".to_string(),
                format!(
                    "Hydrus Client (Rust port)\n\nhydrus version {SOFTWARE_VERSION}\nRust + Qt 6\n\nThis is an in-progress port of the Python client; the query pages, local file import, thumbnails, media viewer and tag editing are wired to the real client database."
                ),
            )],
            "db_info" => {
                let version = self.controller.db().get_version().unwrap_or(0);
                let services = self.controller.services().get_num_services();
                let dir = self.controller.db_dir().display();
                let files = self.query_ids().len();

                vec![UiCommand::Message(
                    "database".to_string(),
                    format!(
                        "database directory: {dir}\nversion: {version}\nservices: {services}\nopen pages: {files}"
                    ),
                )]
            }
            other if other.starts_with("service_info:") => {
                let name = other.trim_start_matches("service_info:");
                let service = self
                    .controller
                    .services()
                    .get_service_key_from_name(name)
                    .and_then(|key| self.controller.services().get_service(key).cloned());

                match service {
                    Some(service) => vec![UiCommand::Message(
                        service.name.clone(),
                        format!(
                            "name: {}\ntype: {:?}\nservice id: {}\nkey: {}",
                            service.name,
                            service.service_type,
                            service.service_id,
                            query::hex(&service.service_key)
                        ),
                    )],
                    None => vec![UiCommand::NotifyMessage(format!(
                        "service '{name}' is missing"
                    ))],
                }
            }
            "manage_services" => vec![UiCommand::NotifyMessage(
                "service management is not ported yet; the local file/tag services are active"
                    .to_string(),
            )],
            "options" => vec![UiCommand::NotifyMessage(
                "the options dialog is not ported yet".to_string(),
            )],
            "options_network" => vec![UiCommand::NotifyMessage(
                "the network settings dialog is not ported yet".to_string(),
            )],
            "tag_siblings" | "tag_parents" => vec![UiCommand::NotifyMessage(
                "tag relation editing is not ported yet".to_string(),
            )],
            "db_check" | "db_vacuum" => vec![UiCommand::NotifyMessage(
                "database maintenance UI is not ported yet".to_string(),
            )],
            "pending" => vec![UiCommand::NotifyMessage(
                "there is no pending upload in this local-only build".to_string(),
            )],
            "pause_network" => vec![UiCommand::NotifyMessage(
                "network pause is not ported yet".to_string(),
            )],
            "undo" => vec![UiCommand::NotifyMessage("nothing to undo".to_string())],
            unknown => vec![UiCommand::NotifyMessage(format!(
                "unknown action: {unknown}"
            ))],
        }
    }

    fn page_id_or_active(&self, target: &str) -> String {
        if target.is_empty() {
            self.active.clone().unwrap_or_default()
        } else {
            target.to_string()
        }
    }

    fn refresh_page_commands(&mut self, page_id: &str) -> Vec<UiCommand> {
        let controller = self.controller.clone();

        let json = {
            let Some(page) = self.find_query_mut(page_id) else {
                return vec![UiCommand::NotifyMessage(
                    "page is not a file page".to_string(),
                )];
            };

            query::refresh_page(&controller, page);
            render::query_state(&controller, page).to_string()
        };

        vec![
            UiCommand::UpdatePage(page_id.to_string(), json),
            self.status_command(),
        ]
    }

    fn replace_query_and_refresh(
        &mut self,
        controller: &Controller,
        page_id: &str,
    ) -> Vec<UiCommand> {
        let json = {
            let Some(page) = self.find_query_mut(page_id) else {
                return vec![UiCommand::NotifyMessage(
                    "page is not a file page".to_string(),
                )];
            };

            query::refresh_page(controller, page);
            render::query_state(controller, page).to_string()
        };

        vec![
            UiCommand::UpdatePage(page_id.to_string(), json),
            self.status_command(),
        ]
    }

    fn refresh_commands(&mut self) -> Vec<UiCommand> {
        let controller = self.controller.clone();
        let ids = self.query_ids();
        let mut commands = Vec::new();

        for id in ids {
            let json = {
                let Some(page) = self.find_query_mut(&id) else {
                    continue;
                };
                query::refresh_page(&controller, page);
                render::query_state(&controller, page).to_string()
            };

            commands.push(UiCommand::UpdatePage(id, json));
        }

        commands.push(self.status_command());
        commands
    }

    fn create_page(&mut self, notebook_id: &str, kind: &str) -> Vec<UiCommand> {
        let controller = self.controller.clone();
        let mut query_to_refresh = None;
        let page_id;

        if kind == "notebook" {
            let new_notebook_id = self.new_id("n");
            let query_id = self.new_id("p");
            let mut notebook = Notebook::new(new_notebook_id, "page of pages");
            notebook.add(PageNode::Query(QueryPage::new(
                query_id.clone(),
                "files",
                "my files",
            )));
            page_id = query_id.clone();
            query_to_refresh = Some(query_id);

            let Some(parent) = self.find_notebook_mut(notebook_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };
            parent.add(PageNode::Notebook(notebook));
        } else if let Some(location) = kind.strip_prefix("query:") {
            let id = self.new_id("p");
            let title = match location {
                "trash" => "trash",
                "all local files" => "all local files",
                _ => "files",
            };
            let node = PageNode::Query(QueryPage::new(id.clone(), title, location));
            page_id = id.clone();
            query_to_refresh = Some(id.clone());

            let Some(parent) = self.find_notebook_mut(notebook_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };
            parent.add(node);
        } else {
            let id = self.new_id("p");
            let (title, message) = placeholder_for(kind);
            page_id = id.clone();

            let Some(parent) = self.find_notebook_mut(notebook_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };
            parent.add(PageNode::Placeholder(PlaceholderPage {
                id,
                title,
                message,
                status: String::new(),
            }));
        }

        if let Some(id) = query_to_refresh {
            if let Some(page) = self.find_query_mut(&id) {
                query::refresh_page(&controller, page);
            }
        }

        self.active = Some(page_id);

        vec![
            UiCommand::BuildPages(render::pages_json(self)),
            self.status_command(),
        ]
    }

    fn close_page(&mut self, page_id: &str) -> Vec<UiCommand> {
        let mut ids = Vec::new();

        if let Some(node) = model::find_node(&self.root, page_id) {
            collect_page_ids(node, &mut ids);
        }

        if !self.remove_page(page_id) {
            return vec![UiCommand::NotifyMessage("page is missing".to_string())];
        }

        let mut commands = Vec::new();

        for id in &ids {
            for viewer in self.close_viewers_for_page(id) {
                commands.push(UiCommand::CloseViewer(viewer));
            }
        }

        if self
            .active
            .as_deref()
            .is_some_and(|active| ids.iter().any(|id| id.as_str() == active))
            || self.active.is_none()
        {
            self.active =
                model::current_leaf(&self.root).or_else(|| model::first_page_id(&self.root));
        }

        commands.push(UiCommand::BuildPages(render::pages_json(self)));
        commands.push(self.status_command());

        commands
    }

    fn close_other_pages(&mut self, page_id: &str) -> Vec<UiCommand> {
        let Some(parent_id) = model::parent_notebook_id(&self.root, page_id) else {
            return vec![UiCommand::NotifyMessage("page is missing".to_string())];
        };

        let Some(parent) = self.find_notebook_mut(&parent_id) else {
            return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
        };

        let mut removed = Vec::new();

        parent.children.retain(|child| {
            if child.id() == page_id {
                true
            } else {
                collect_page_ids(child, &mut removed);
                false
            }
        });
        parent.current = Some(page_id.to_string());

        let mut commands = Vec::new();

        for id in removed {
            for viewer in self.close_viewers_for_page(&id) {
                commands.push(UiCommand::CloseViewer(viewer));
            }
        }

        self.active = Some(page_id.to_string());
        commands.push(UiCommand::BuildPages(render::pages_json(self)));
        commands.push(self.status_command());

        commands
    }

    fn rename_page(&mut self, page_id: &str, name: &str) -> Vec<UiCommand> {
        if name.is_empty() {
            return vec![];
        }

        let Some(node) = model::find_node_mut(&mut self.root, page_id) else {
            return vec![UiCommand::NotifyMessage("page is missing".to_string())];
        };

        match node {
            PageNode::Query(page) => page.title = name.to_string(),
            PageNode::Notebook(notebook) => notebook.title = name.to_string(),
            PageNode::Placeholder(page) => page.title = name.to_string(),
        }

        vec![
            UiCommand::BuildPages(render::pages_json(self)),
            self.status_command(),
        ]
    }

    fn focus(&mut self, page_id: &str, hash_hex: &str) -> Vec<UiCommand> {
        let controller = self.controller.clone();

        let Some(hash) = query::unhex(hash_hex) else {
            return vec![];
        };

        let (tags_json, preview_path) = {
            let Some(page) = self.find_query_mut(page_id) else {
                return vec![];
            };

            page.focused = Some(hash.clone());

            let tags = query::selection_tags(&controller, &hash);
            let preview = query::file_path(&controller, &hash)
                .map(|path| path.display().to_string())
                .unwrap_or_default();

            (
                serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string()),
                preview,
            )
        };

        vec![
            UiCommand::SetSelection {
                page_id: page_id.to_string(),
                hash: query::hex(&hash),
                tags_json,
                preview_path,
            },
            self.status_command(),
        ]
    }

    fn open_viewer(&mut self, page_id: &str, hash_hex: &str) -> Vec<UiCommand> {
        let controller = self.controller.clone();

        let Some(hash) = query::unhex(hash_hex) else {
            return vec![];
        };

        let viewer_id = self.new_id("v");

        let Some(page) = self.find_query(page_id) else {
            return vec![UiCommand::NotifyMessage("page is missing".to_string())];
        };

        let json = render::viewer_json(&controller, &viewer_id, page, &hash);

        self.viewers.insert(
            viewer_id.clone(),
            ViewerContext {
                page_id: page_id.to_string(),
                hash: hash.clone(),
            },
        );

        if let Some(page) = self.find_query_mut(page_id) {
            page.focused = Some(hash);
        }

        vec![UiCommand::OpenViewer(json)]
    }

    fn viewer_nav(&mut self, viewer_id: &str, direction: &str) -> Vec<UiCommand> {
        let controller = self.controller.clone();

        let Some(context) = self.viewers.get(viewer_id).cloned() else {
            return vec![];
        };

        let next = {
            let Some(page) = self.find_query(&context.page_id) else {
                return vec![];
            };

            query::neighbour(page, &context.hash, direction)
        };

        let Some(next) = next else {
            return vec![];
        };

        self.viewers.insert(
            viewer_id.to_string(),
            ViewerContext {
                page_id: context.page_id.clone(),
                hash: next.clone(),
            },
        );

        let json = {
            let Some(page) = self.find_query(&context.page_id) else {
                return vec![];
            };

            render::viewer_json(&controller, viewer_id, page, &next)
        };

        let mut commands = vec![UiCommand::UpdateViewer(viewer_id.to_string(), json)];

        if self.active.as_deref() == Some(context.page_id.as_str()) {
            commands.extend(self.focus(&context.page_id, &query::hex(&next)));
        }

        commands
    }

    fn manage_tags(&mut self, page_id: &str, hash_hex: &str, active: bool) -> Vec<UiCommand> {
        let controller = self.controller.clone();

        let (page_id, hash_hex) = if active {
            let Some(page) = self.active_query() else {
                return vec![UiCommand::NotifyMessage("no page selected".to_string())];
            };
            let Some(hash) = page.focused.clone() else {
                return vec![UiCommand::NotifyMessage("no file selected".to_string())];
            };

            (page.id.clone(), query::hex(&hash))
        } else {
            (page_id.to_string(), hash_hex.to_string())
        };

        let Some(hash) = query::unhex(&hash_hex) else {
            return vec![];
        };

        let tags = query::selection_tags(&controller, &hash);
        let service = controller.services().get_name(b"local tags");
        let json = json!({
            "dialog_id": "tags",
            "hash": hash_hex,
            "service": service,
            "tags": tags,
            "page_id": page_id,
        })
        .to_string();

        vec![UiCommand::TagDialog(json)]
    }

    fn apply_tags(&mut self, payload: &str) -> Vec<UiCommand> {
        let Ok(value) = serde_json::from_str::<Value>(payload) else {
            return vec![UiCommand::NotifyMessage("invalid tag payload".to_string())];
        };

        let Some(hash) = value
            .get("hash")
            .and_then(Value::as_str)
            .and_then(query::unhex)
        else {
            return vec![UiCommand::NotifyMessage("invalid file hash".to_string())];
        };

        let Some(hash_id) = query::hash_id_for(&self.controller, &hash) else {
            return vec![UiCommand::NotifyMessage(
                "file is not in the database".to_string(),
            )];
        };

        let Some(service_id) = query::tag_service_id(&self.controller) else {
            return vec![UiCommand::NotifyMessage("no local tag service".to_string())];
        };

        let db = self.controller.db().db();
        let mut changed = 0;

        if let Some(values) = value.get("add").and_then(Value::as_array) {
            for tag in values.iter().filter_map(Value::as_str) {
                if let Ok(tag_id) = master_tags::get_tag_id(db, tag) {
                    let _ = mappings::add_mappings(
                        db,
                        service_id,
                        ContentStatus::Current,
                        hash_id,
                        &[tag_id],
                    );
                    changed += 1;
                }
            }
        }

        if let Some(values) = value.get("remove").and_then(Value::as_array) {
            for tag in values.iter().filter_map(Value::as_str) {
                if let Ok(tag_id) = master_tags::get_tag_id(db, tag) {
                    let _ = mappings::remove_mappings(
                        db,
                        service_id,
                        ContentStatus::Current,
                        hash_id,
                        &[tag_id],
                    );
                    changed += 1;
                }
            }
        }

        self.message = format!("tags updated ({changed} changes)");

        let mut commands = self.refresh_commands();

        if let Some(page) = self.active_query() {
            if let Some(hash) = page.focused.clone() {
                commands.extend(self.focus(&page.id.clone(), &query::hex(&hash)));
            }
        }

        let viewer_updates = self.viewer_updates_for_hash(&hash);
        commands.extend(viewer_updates);
        commands.push(self.status_command());

        commands
    }

    fn viewer_updates_for_hash(&self, hash: &[u8]) -> Vec<UiCommand> {
        let controller = &self.controller;
        let mut commands = Vec::new();

        for (viewer_id, context) in &self.viewers {
            if context.hash != hash {
                continue;
            }

            if let Some(page) = self.find_query(&context.page_id) {
                commands.push(UiCommand::UpdateViewer(
                    viewer_id.clone(),
                    render::viewer_json(controller, viewer_id, page, hash),
                ));
            }
        }

        commands
    }

    fn set_inbox(&mut self, page_id: &str, hash_hex: &str, inbox: bool) -> Vec<UiCommand> {
        let Some(hash) = query::unhex(hash_hex) else {
            return vec![];
        };

        let Some(hash_id) = query::hash_id_for(&self.controller, &hash) else {
            return vec![];
        };

        let _ = files_storage::set_inbox(self.controller.db().db(), &[hash_id], inbox);

        self.message = if inbox {
            "file moved to inbox".to_string()
        } else {
            "file archived".to_string()
        };

        let _ = page_id;
        self.refresh_commands()
    }

    fn set_inbox_active(&mut self, inbox: bool) -> Vec<UiCommand> {
        let Some(page) = self.active_query() else {
            return vec![UiCommand::NotifyMessage("no page selected".to_string())];
        };

        let Some(hash) = page.focused.clone() else {
            return vec![UiCommand::NotifyMessage("no file selected".to_string())];
        };

        let page_id = page.id.clone();
        self.set_inbox(&page_id, &query::hex(&hash), inbox)
    }

    fn delete_files(&mut self, page_id: &str, hash_hex: &str) -> Vec<UiCommand> {
        let Some(hash) = query::unhex(hash_hex) else {
            return vec![];
        };

        let Some(hash_id) = query::hash_id_for(&self.controller, &hash) else {
            return vec![];
        };

        let db = self.controller.db().db();
        let my_files = query::local_file_service_id(&self.controller, "my files");

        if let Some(trash) = query::trash_service_id(&self.controller) {
            let _ = files_storage::add_files(
                db,
                trash,
                ContentStatus::Current,
                &[hash_id],
                Some(hydrus_time::get_now()),
            );
        }

        if let Some(service) = my_files {
            let _ = files_storage::remove_files(db, service, ContentStatus::Current, &[hash_id]);
        }

        let _ = files_storage::set_inbox(db, &[hash_id], false);

        self.message = "file moved to trash".to_string();
        let _ = page_id;
        self.refresh_commands()
    }

    fn import_paths(&mut self, payload: &str) -> Vec<UiCommand> {
        let paths: Vec<PathBuf> = serde_json::from_str::<Vec<String>>(payload)
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect();

        if paths.is_empty() {
            return vec![UiCommand::NotifyMessage("no files selected".to_string())];
        }

        let summary = import::import_paths(&self.controller, &paths);
        self.message = summary.message();

        if self.query_ids().is_empty() {
            let id = self.new_id("p");
            self.root.add(PageNode::Query(QueryPage::new(
                id.clone(),
                "files",
                "my files",
            )));
            self.active = Some(id);
            self.refresh_all();

            return vec![
                UiCommand::BuildPages(render::pages_json(self)),
                self.status_command(),
            ];
        }

        self.refresh_commands()
    }
}

fn refresh_notebook(notebook: &mut Notebook, controller: &Controller) {
    for child in &mut notebook.children {
        match child {
            PageNode::Query(page) => query::refresh_page(controller, page),
            PageNode::Notebook(inner) => refresh_notebook(inner, controller),
            PageNode::Placeholder(_) => {}
        }
    }
}

fn collect_query_ids(notebook: &Notebook, ids: &mut Vec<String>) {
    for child in &notebook.children {
        match child {
            PageNode::Query(page) => ids.push(page.id.clone()),
            PageNode::Notebook(inner) => collect_query_ids(inner, ids),
            PageNode::Placeholder(_) => {}
        }
    }
}

fn collect_page_ids(node: &PageNode, ids: &mut Vec<String>) {
    ids.push(node.id().to_string());

    if let PageNode::Notebook(notebook) = node {
        for child in &notebook.children {
            collect_page_ids(child, ids);
        }
    }
}

fn placeholder_for(kind: &str) -> (String, String) {
    match kind {
        "duplicates" => (
            "duplicates processing".to_string(),
            "Duplicates processing\n\nThe duplicate-pair queue and merge UI are not ported yet. File pages, import, thumbnails, the media viewer and tag editing are functional in this build.".to_string(),
        ),
        "urls" => (
            "urls".to_string(),
            "URL downloader\n\nThe network downloader page is not ported yet. Use File > Import files for local files.".to_string(),
        ),
        "watcher" => (
            "watcher".to_string(),
            "Thread watcher\n\nThe watcher/downloader page is not ported yet. Use File > Import files for local files.".to_string(),
        ),
        "gallery" => (
            "gallery".to_string(),
            "Gallery downloader\n\nThe gallery downloader page is not ported yet. Use File > Import files for local files.".to_string(),
        ),
        "simple" => (
            "simple downloader".to_string(),
            "Simple downloader\n\nThe simple downloader page is not ported yet. Use File > Import files for local files.".to_string(),
        ),
        _ => (
            "page".to_string(),
            "This page type is not ported yet.".to_string(),
        ),
    }
}
