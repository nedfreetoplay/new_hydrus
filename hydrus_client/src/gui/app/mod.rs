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

pub mod auto_resolution;
pub mod commands;
pub mod download;
pub mod duplicates;
pub mod import;
pub mod import_folders;
pub mod metadata;
pub mod model;
pub mod options;
pub mod query;
pub mod render;
pub mod session;
pub mod sessions;
pub mod tag_relations;
pub mod worker;

pub use commands::UiCommand;
use model::{
    AppState, DownloaderKind, DownloaderPage, DuplicatesPage, Notebook, PageNode, PlaceholderPage,
    QueryPage, SubscriptionsPage, ViewerContext,
};

thread_local! {
    static STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

pub fn install(controller: Arc<Controller>) {
    let state = AppState::new(controller);
    STATE.with(|slot| *slot.borrow_mut() = Some(state));

    with_state_mut(|state| {
        state.refresh_all();

        match state.controller.refresh_api() {
            Ok(status) if status != "api disabled" => state.message = status,
            Ok(_) => {}
            Err(error) => state.message = format!("api failed: {error}"),
        }
    });
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

/// Qt timer tick: drains background download/import events and runs due
/// watcher/subscription checks.
pub fn handle_tick(mut window: Pin<&mut ffi::HydrusMainWindow>) {
    let commands = with_state_mut(download::tick);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

/// Stops the API server and saves the page tree when the application exits.
pub fn shutdown() {
    save_session();

    with_state(|state| {
        state.controller.stop_api();
    });
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

    pub(crate) fn refresh_all(&mut self) {
        let controller = self.controller.clone();
        let rules = self.auto_rules.clone();
        refresh_notebook(&mut self.root, &controller, &rules);
    }

    pub(crate) fn query_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        collect_query_ids(&self.root, &mut ids);
        ids
    }

    fn dispatch(&mut self, action: &str, target: &str, payload: &str) -> Vec<UiCommand> {
        match action {
            "page_selected" => {
                self.active = Some(target.to_string());
                self.record_history(target);
                let mut commands = vec![self.status_command()];

                if self.select_page(target) {
                    commands.insert(0, UiCommand::Menus(render::menus_json(self)));
                }

                commands
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
            "new_urls" => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "urls")
            }
            "new_gallery" => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "gallery")
            }
            "new_watcher" => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "watcher")
            }
            "new_simple" => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "simple")
            }
            "new_subscriptions" => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "subscriptions")
            }
            "new_import_folders" | "import_folders" => {
                let notebook = self.active_notebook_id();
                import_folders::create_page(self, &notebook)
            }
            "import_folder_add" => import_folders::add(self, payload),
            "import_folder_edit" => import_folders::edit(self, payload),
            "import_folder_save" => import_folders::save_action(self, payload),
            "import_folder_remove" => import_folders::remove(self, payload),
            "import_folder_toggle" => import_folders::toggle(self, payload),
            "import_folder_check" => import_folders::check_now(self, payload),
            "create_page" => self.create_page(target, payload),
            "close_page" => self.close_page(target),
            "close_other_pages" => self.close_other_pages(target),
            "move_page" => self.page_move(target, payload),
            "duplicate_page" => self.duplicate_page_command(target),
            "history_back" => self.navigate_history(true),
            "history_forward" => self.navigate_history(false),
            "page_manager" => vec![UiCommand::PageManagerDialog(render::page_manager_json(
                self,
            ))],
            "page_manager_activate" => {
                if self.select_page(target) {
                    vec![
                        UiCommand::BuildPages(render::pages_json(self)),
                        UiCommand::Menus(render::menus_json(self)),
                        self.status_command(),
                    ]
                } else {
                    vec![UiCommand::NotifyMessage("page is missing".to_string())]
                }
            }
            "page_manager_close" => {
                let ids: Vec<String> = serde_json::from_str(payload).unwrap_or_default();
                let mut commands = Vec::new();

                for id in ids {
                    commands.extend(self.close_page(&id));
                }

                commands
            }
            "session_save_as" => vec![UiCommand::PromptText {
                action: "session_save".to_string(),
                title: "save session".to_string(),
                label: "session name".to_string(),
                initial: self.session_name.clone().unwrap_or_default(),
            }],
            "session_save" => match sessions::save_named(self, payload) {
                Ok(name) => {
                    self.message = format!("saved session '{name}'");
                    vec![
                        UiCommand::Menus(render::menus_json(self)),
                        self.status_command(),
                    ]
                }
                Err(error) => vec![UiCommand::Message(
                    "save session".to_string(),
                    format!("could not save the session: {error}"),
                )],
            },
            "session_backup" => match sessions::backup(self) {
                Ok(name) => {
                    self.message = format!("backed up session as '{name}'");
                    vec![
                        UiCommand::Menus(render::menus_json(self)),
                        self.status_command(),
                    ]
                }
                Err(error) => vec![UiCommand::Message(
                    "backup session".to_string(),
                    format!("could not back up the session: {error}"),
                )],
            },
            "session_load" => self.load_session_named(target, false, false),
            "session_append" => self.load_session_named(target, true, false),
            "session_append_backup" => self.load_session_named(target, true, true),
            "session_delete" => vec![UiCommand::ConfirmAction {
                action: "session_delete_confirmed".to_string(),
                target: target.to_string(),
                title: "delete session".to_string(),
                text: format!("Delete the session '{target}'? This cannot be undone."),
            }],
            "session_delete_confirmed" => match sessions::delete_named(self, target) {
                Ok(()) => {
                    self.message = format!("deleted session '{target}'");
                    vec![
                        UiCommand::Menus(render::menus_json(self)),
                        self.status_command(),
                    ]
                }
                Err(error) => vec![UiCommand::Message(
                    "delete session".to_string(),
                    format!("could not delete the session: {error}"),
                )],
            },
            "rename_page" => self.rename_page(target, payload),
            "refresh_page" => {
                let id = self.page_id_or_active(target);
                self.refresh_page_commands(&id)
            }
            "search" => {
                let controller = self.controller.clone();

                if let Some(page) = self.find_query_mut(target) {
                    page.query = payload.to_string();
                    page.fixed_hashes = None;
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
            "tag_add" => metadata::tag_action(self, true, payload),
            "tag_remove" => metadata::tag_action(self, false, payload),
            "rating_set" => metadata::rating_action(self, payload),
            "notes_manage" | "manage_notes_active" => {
                if action == "manage_notes_active" {
                    metadata::manage_notes_active(self)
                } else {
                    metadata::notes_action(self, payload, false)
                }
            }
            "notes_save" => metadata::notes_action(self, payload, true),
            "apply_tags" => self.apply_tags(payload),
            "archive_files" => self.set_inbox(target, payload, false),
            "inbox_files" => self.set_inbox(target, payload, true),
            "archive_active" => self.set_inbox_active(false),
            "inbox_active" => self.set_inbox_active(true),
            "delete_files" => self.delete_files(target, payload),
            "import_files" => vec![UiCommand::PickFiles],
            "import_url" => vec![UiCommand::PromptUrl],
            "download_url" => self.download_one_url(payload),
            "import_paths" => self.import_paths(payload),
            "downloader_start" => download::start(self, target, payload),
            "downloader_pause" => download::pause(self, target),
            "downloader_clear" => download::clear(self, target),
            "downloader_retry" => download::retry(self, target, payload),
            "downloader_open_results" => download::open_results(self, target),
            "watcher_check" => download::start(self, target, payload),
            "watcher_auto" => {
                if let Some(page) = self.find_downloader_mut(target) {
                    page.auto_watch = !page.auto_watch;
                    page.status = if page.auto_watch {
                        "watching".to_string()
                    } else {
                        "automatic checks off".to_string()
                    };
                }
                self.downloader_update(target)
            }
            "subscription_check" => download::start_subscription(self, target, payload),
            "subscription_toggle" => download::toggle_subscription(self, target, payload),
            "subscription_delete" => download::delete_subscription(self, target, payload),
            "subscription_add" => download::add_subscription(self, target, payload),
            "subscription_open_results" => {
                download::open_subscription_results(self, target, payload)
            }
            "duplicates_rules" => vec![UiCommand::DuplicatesRulesDialog(
                auto_resolution::dialog_json(self),
            )],
            "duplicates_rule_add" => auto_resolution::add_rule(self, payload),
            "duplicates_rule_remove" => auto_resolution::remove_rule(self, payload),
            "duplicates_auto_toggle" => {
                let mut options = self.controller.options();
                let enabled = !options.get_boolean("duplicates_auto_resolution_enabled");
                options.set_boolean("duplicates_auto_resolution_enabled", enabled);
                self.controller.replace_options(options);
                let _ = self.controller.save_options();
                self.message = if enabled {
                    "duplicate auto-resolution enabled".to_string()
                } else {
                    "duplicate auto-resolution disabled".to_string()
                };
                self.refresh_all();
                let mut commands = self.refresh_commands();
                commands.push(UiCommand::DuplicatesRulesDialog(
                    auto_resolution::dialog_json(self),
                ));
                commands.push(UiCommand::Status(render::status_json(self)));
                commands
            }
            action if action.starts_with("duplicates_") => {
                duplicates::action(self, target, action, payload)
            }
            "options" => vec![UiCommand::OptionsDialog(options::options_json(self))],
            "options_apply" => options::apply_options(self, payload),
            "manage_services" => vec![UiCommand::ServicesDialog(options::services_json(self))],
            "service_add" | "service_rename" | "service_delete" => {
                options::service_action(self, action, payload)
            }
            "options_network" => vec![UiCommand::NetworkDialog(options::network_json(self))],
            action if action.starts_with("network_") => {
                options::network_action(self, action, payload)
            }
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
                    "Hydrus Client (Rust port)\n\nhydrus version {SOFTWARE_VERSION}\nRust + Qt 6\n\nQuery pages, local and URL import, gallery/watcher/subscriptions downloaders, duplicates processing, thumbnails, the media viewer, tag editing, options, service/network managers and the local Client API are wired to the real client database."
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
            "pause_network" => options::network_action(self, "network_pause", ""),
            "tag_siblings" => tag_relations::show(self, tag_relations::SIBLINGS),
            "tag_parents" => tag_relations::show(self, tag_relations::PARENTS),
            "tag_relations_add" => tag_relations::action(self, payload, true),
            "tag_relations_remove" => tag_relations::action(self, payload, false),
            "db_check" => {
                let result =
                    self.controller
                        .db()
                        .db()
                        .query_one("PRAGMA quick_check", &[], |row| row.get::<_, String>(0));

                match result {
                    Ok(status) => vec![UiCommand::Message(
                        "database check".to_string(),
                        format!(
                            "sqlite quick_check: {}",
                            status.unwrap_or_else(|| "no result".to_string())
                        ),
                    )],
                    Err(error) => vec![UiCommand::Message(
                        "database check".to_string(),
                        format!("database check failed: {error}"),
                    )],
                }
            }
            "db_vacuum" => {
                let result = self.controller.db().db().execute_batch("VACUUM;");

                match result {
                    Ok(()) => vec![UiCommand::Message(
                        "database vacuum".to_string(),
                        "the database was vacuumed successfully".to_string(),
                    )],
                    Err(error) => vec![UiCommand::Message(
                        "database vacuum".to_string(),
                        format!("vacuum failed: {error}"),
                    )],
                }
            }
            "pending" => vec![UiCommand::NotifyMessage(
                "there is no pending upload in this local-only build".to_string(),
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

    pub(crate) fn refresh_page_commands(&mut self, page_id: &str) -> Vec<UiCommand> {
        let controller = self.controller.clone();

        if let Some(page) = self.find_query_mut(page_id) {
            query::refresh_page(&controller, page);
            let json = render::query_state(&controller, page).to_string();

            return vec![
                UiCommand::UpdatePage(page_id.to_string(), json),
                self.status_command(),
            ];
        }

        if self.find_downloader(page_id).is_some() {
            return self.downloader_update(page_id);
        }

        if let Some(page) = self.find_duplicates_mut(page_id) {
            let value = render::duplicates_json(&controller, page);
            return vec![
                UiCommand::UpdateDuplicates(page_id.to_string(), value),
                self.status_command(),
            ];
        }

        if self.find_subscriptions(page_id).is_some() {
            return vec![
                UiCommand::UpdateSubscriptions(
                    page_id.to_string(),
                    render::subscriptions_json(self, page_id),
                ),
                self.status_command(),
            ];
        }

        if self.find_import_folders(page_id).is_some() {
            return vec![
                UiCommand::UpdateImportFolders(
                    page_id.to_string(),
                    import_folders::rules_json(self, page_id),
                ),
                self.status_command(),
            ];
        }

        vec![UiCommand::NotifyMessage(
            "this page cannot be refreshed".to_string(),
        )]
    }

    pub(crate) fn downloader_update(&self, page_id: &str) -> Vec<UiCommand> {
        match self.find_downloader(page_id) {
            Some(page) => vec![
                UiCommand::UpdateDownloader(
                    page_id.to_string(),
                    render::downloader_json(&self.controller, page),
                ),
                self.status_command(),
            ],
            None => vec![],
        }
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

    pub(crate) fn refresh_commands(&mut self) -> Vec<UiCommand> {
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
        } else if kind == "subscriptions" {
            let id = self.new_id("p");
            page_id = id.clone();

            let Some(parent) = self.find_notebook_mut(notebook_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };
            parent.add(PageNode::Subscriptions(SubscriptionsPage::new(id)));
        } else if kind == "import_folders" {
            return import_folders::create_page(self, notebook_id);
        } else if kind == "duplicates" {
            let id = self.new_id("p");
            page_id = id.clone();
            let mut page = DuplicatesPage::new(id);
            duplicates::refresh_for(&controller, &mut page, &self.auto_rules);

            let Some(parent) = self.find_notebook_mut(notebook_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };
            parent.add(PageNode::Duplicates(page));
        } else if let Some(kind) = DownloaderKind::parse(kind) {
            let id = self.new_id("p");
            page_id = id.clone();
            let mut page = DownloaderPage::new(id, kind);

            if kind == DownloaderKind::Watcher {
                let period = controller.options().get_integer("watcher_check_period");

                page.check_period = if period >= 60 {
                    period.clamp(60, 30 * 86_400)
                } else {
                    3600
                };
            }

            let Some(parent) = self.find_notebook_mut(notebook_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };
            parent.add(PageNode::Downloader(page));
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

        self.active = Some(page_id.clone());

        self.record_history(&page_id);

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

        let mut known = Vec::new();
        for child in &self.root.children {
            collect_page_ids(child, &mut known);
        }
        self.history
            .retain(|id| known.iter().any(|known_id| known_id == id));

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

        let mut known = Vec::new();
        for child in &self.root.children {
            collect_page_ids(child, &mut known);
        }
        self.history
            .retain(|id| known.iter().any(|known_id| known_id == id));

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

    fn page_move(&mut self, page_id: &str, payload: &str) -> Vec<UiCommand> {
        let request: Value = match serde_json::from_str(payload) {
            Ok(value) => value,
            Err(_) => {
                return vec![UiCommand::NotifyMessage(
                    "move request is invalid".to_string(),
                )];
            }
        };

        let target = request
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let position = request
            .get("position")
            .and_then(Value::as_str)
            .unwrap_or("after");

        match self.move_page(page_id, target, position) {
            Ok(()) => {
                self.record_history(page_id);
                vec![
                    UiCommand::BuildPages(render::pages_json(self)),
                    UiCommand::Menus(render::menus_json(self)),
                    self.status_command(),
                ]
            }
            Err(error) => vec![UiCommand::NotifyMessage(error)],
        }
    }

    fn duplicate_page_command(&mut self, page_id: &str) -> Vec<UiCommand> {
        match self.duplicate_page(page_id) {
            Ok(id) => {
                self.record_history(&id);
                vec![
                    UiCommand::BuildPages(render::pages_json(self)),
                    UiCommand::Menus(render::menus_json(self)),
                    self.status_command(),
                ]
            }
            Err(error) => vec![UiCommand::NotifyMessage(error)],
        }
    }

    fn navigate_history(&mut self, back: bool) -> Vec<UiCommand> {
        let target = if back {
            self.history_back()
        } else {
            self.history_forward()
        };

        match target {
            Some(id) => {
                self.select_page(&id);
                vec![
                    UiCommand::BuildPages(render::pages_json(self)),
                    UiCommand::Menus(render::menus_json(self)),
                    self.status_command(),
                ]
            }
            None => vec![self.status_command()],
        }
    }

    fn load_session_named(&mut self, name: &str, append: bool, backup: bool) -> Vec<UiCommand> {
        let loaded = if backup {
            sessions::load_backup(self, name)
        } else {
            sessions::load_named(self, name)
        };

        let loaded = match loaded {
            Ok(loaded) => loaded,
            Err(error) => {
                return vec![UiCommand::Message("load session".to_string(), error)];
            }
        };

        if append {
            let added = self.append_session(loaded);
            self.refresh_all();
            self.message = format!("appended {added} page groups from '{name}'");
            return vec![
                UiCommand::Menus(render::menus_json(self)),
                UiCommand::BuildPages(render::pages_json(self)),
                self.status_command(),
            ];
        }

        let viewer_ids: Vec<String> = self.viewers.keys().cloned().collect();
        let restored = loaded
            .session_name
            .clone()
            .unwrap_or_else(|| name.to_string());
        self.replace_session(loaded);
        self.session_name = Some(restored);
        self.refresh_all();

        let mut commands: Vec<UiCommand> =
            viewer_ids.into_iter().map(UiCommand::CloseViewer).collect();

        self.message = format!("loaded session '{name}'");
        commands.push(UiCommand::Menus(render::menus_json(self)));
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
            PageNode::Downloader(page) => page.title = name.to_string(),
            PageNode::Subscriptions(page) => page.title = name.to_string(),
            PageNode::Duplicates(page) => page.title = name.to_string(),
            PageNode::ImportFolders(page) => page.title = name.to_string(),
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

        let (tags_json, preview_path, metadata_json) = {
            let Some(page) = self.find_query_mut(page_id) else {
                return vec![];
            };

            page.focused = Some(hash.clone());

            let tags = query::selection_tags(&controller, &hash);
            let preview = query::preview_path(&controller, &hash)
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            let metadata = metadata::metadata_json(&controller, &hash);

            (
                serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string()),
                preview,
                metadata.to_string(),
            )
        };

        vec![
            UiCommand::SetSelection {
                page_id: page_id.to_string(),
                hash: query::hex(&hash),
                tags_json,
                preview_path,
                metadata_json,
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
            "metadata": metadata::metadata_json(&controller, &hash),
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

        let Some(service_id) = value
            .get("service")
            .and_then(Value::as_str)
            .and_then(|service| metadata::tag_service_id(&self.controller, service))
            .or_else(|| query::tag_service_id(&self.controller))
        else {
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

        let mut commands = metadata::refresh_hash(self, &hash);
        commands.push(self.status_command());

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

        let job = worker::AsyncJob::ImportPaths {
            paths: paths.clone(),
            folder: None,
        };

        if let Err(error) = self.runtime.submit(job) {
            return vec![UiCommand::NotifyMessage(error)];
        }

        self.import_queue = model::ImportQueueState {
            running: true,
            done: 0,
            total: paths.len(),
            imported: 0,
            failed: 0,
            message: format!("queued {} files", paths.len()),
        };
        self.message = self.import_queue.message.clone();

        let mut commands = Vec::new();

        if self.query_ids().is_empty() {
            let id = self.new_id("p");
            self.root.add(PageNode::Query(QueryPage::new(
                id.clone(),
                "files",
                "my files",
            )));
            self.active = Some(id);
            self.refresh_all();
            commands.push(UiCommand::BuildPages(render::pages_json(self)));
        }

        commands.push(self.status_command());
        commands
    }

    fn download_one_url(&mut self, url: &str) -> Vec<UiCommand> {
        let url = url.trim();

        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return vec![UiCommand::NotifyMessage(
                "the url must start with http:// or https://".to_string(),
            )];
        }

        let notebook = self.active_notebook_id();
        let id = self.new_id("p");
        let mut page = DownloaderPage::new(id.clone(), DownloaderKind::Urls);
        page.title = "url import".to_string();
        page.input = url.to_string();

        let Some(parent) = self.find_notebook_mut(&notebook) else {
            return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
        };
        parent.add(PageNode::Downloader(page));
        self.active = Some(id.clone());

        let payload = serde_json::json!({"input": url}).to_string();
        let mut commands = vec![UiCommand::BuildPages(render::pages_json(self))];
        commands.extend(download::start(self, &id, &payload));
        commands
    }
}

fn refresh_notebook(
    notebook: &mut Notebook,
    controller: &Controller,
    rules: &[auto_resolution::AutoResolutionRule],
) {
    for child in &mut notebook.children {
        match child {
            PageNode::Query(page) => query::refresh_page(controller, page),
            PageNode::Notebook(inner) => refresh_notebook(inner, controller, rules),
            PageNode::Duplicates(page) => duplicates::refresh_for(controller, page, rules),
            PageNode::Downloader(_)
            | PageNode::Subscriptions(_)
            | PageNode::ImportFolders(_)
            | PageNode::Placeholder(_) => {}
        }
    }
}

fn collect_query_ids(notebook: &Notebook, ids: &mut Vec<String>) {
    for child in &notebook.children {
        match child {
            PageNode::Query(page) => ids.push(page.id.clone()),
            PageNode::Notebook(inner) => collect_query_ids(inner, ids),
            PageNode::Downloader(_)
            | PageNode::Subscriptions(_)
            | PageNode::Duplicates(_)
            | PageNode::ImportFolders(_)
            | PageNode::Placeholder(_) => {}
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

fn collect_import_folder_ids(notebook: &Notebook, ids: &mut Vec<String>) {
    for child in &notebook.children {
        match child {
            PageNode::ImportFolders(page) => ids.push(page.id.clone()),
            PageNode::Notebook(inner) => collect_import_folder_ids(inner, ids),
            _ => {}
        }
    }
}

fn placeholder_for(kind: &str) -> (String, String) {
    (
        kind.to_string(),
        format!("Unknown page type '{kind}'.\n\nThe page was kept so it can be renamed or closed."),
    )
}
