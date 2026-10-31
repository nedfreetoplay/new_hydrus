//! Application state and event handling for the real client GUI.

use std::cell::RefCell;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use hydrus_core::hydrus_constants::{ContentStatus, TimestampType, dirs};
use hydrus_core::hydrus_time;
use serde_json::{Value, json};

use crate::controller::Controller;
use crate::db::{files_storage, files_timestamps, files_viewing_stats, mappings, master_tags};
use crate::gui::qt::bridge::ffi;
use crate::gui::qt::panels;

pub mod api_permissions;
pub mod auto_resolution;
pub mod backup;
pub mod commands;
pub mod download;
pub mod downloader_export;
pub mod duplicates;
pub mod export;
pub mod favourites;
pub mod file_locations;
pub mod import;
pub mod import_folders;
pub mod import_options;
pub mod local_import;
pub mod metadata;
pub mod model;
pub mod notifications;
pub mod options;
pub mod orphans;
pub mod query;
pub mod render;
pub mod search;
pub mod search_history;
pub mod seeds;
pub mod services;
pub mod session;
pub mod sessions;
pub mod tag_filter;
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

        let mut commands = vec![
            UiCommand::Menus(render::menus_json(state)),
            UiCommand::TagSuggestions(tags),
            UiCommand::BuildPages(render::pages_json(state)),
            UiCommand::Status(render::status_json(state)),
        ];

        if state
            .controller
            .files_manager()
            .subfolders()
            .iter()
            .any(|subfolder| !subfolder.path_exists(false))
        {
            commands.extend(file_locations::open(state));
        }

        commands
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

/// Development helper: attaches a source URL to the first file of the active page.
pub fn add_url_for_testing(url: &str) {
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

        let _ = crate::db::url_map::add_url(state.controller.db().db(), hash_id, url);
        state.message = format!("added url {url}");
    });
}

/// Development helper: leaves file pages in their searching state so the
/// loading panel can be checked in screenshots.
pub fn show_page_loading_for_testing() {
    with_state_mut(|state| {
        for id in state.query_ids() {
            if let Some(page) = state.find_query_mut(&id) {
                page.loading = true;
            }
        }
    });
}

/// Development helper: sets the thumbnail layout options before the first page
/// is built so the results panel can be checked in screenshots.
pub fn set_thumbnail_options_for_testing(scale_percent: i64, masonry: bool, horizontal: bool) {
    with_state_mut(|state| {
        let mut options = state.controller.options();
        options.set_integer("thumbnail_scale_percent", scale_percent.clamp(25, 400));
        options.set_boolean("thumbnail_layout_masonry", masonry);
        options.set_boolean("thumbnail_scroll_horizontal", horizontal);
        state.controller.replace_options(options);
    });
}

/// Development helper: records views on the first file of the active page.
pub fn add_viewing_stats_for_testing(views: i64) {
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

        let _ = files_viewing_stats::add_viewing_stats(
            state.controller.db().db(),
            hash_id,
            0,
            views,
            1500,
            Some(hydrus_time::get_now_ms()),
        );
        state.message = format!("recorded {views} views");
    });
}

/// Development helper: applies several tags to the first file on the active page.
pub fn add_tags_for_testing(tags: &[String]) {
    for tag in tags {
        add_tag_for_testing(tag);
    }
}

/// Development helper: changes tag presentation options before the first page
/// is rendered.
pub fn set_tag_display_options_for_testing(
    sort: &str,
    show_namespaces: bool,
    replace_underscores: bool,
) {
    with_state_mut(|state| {
        let mut options = state.controller.options();
        options.set_string("tag_display_sort", sort);
        options.set_boolean("show_namespaces", show_namespaces);
        options.set_boolean("replace_tag_underscores_with_spaces", replace_underscores);
        state.controller.replace_options(options);
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

    with_state_mut(|state| {
        for command in &commands {
            if let UiCommand::NotifyMessage(text) = command {
                state.notifications.push("info", text);
                let limit = state
                    .controller
                    .options()
                    .get_integer("popup_message_max_messages")
                    .clamp(1, 5_000) as usize;
                state.notifications.trim_to(limit);
            }
        }
    });

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

    let commands = with_state_mut(maintenance_tick);

    for command in commands {
        panels::execute(window.as_mut(), command);
    }
}

/// Runs a couple of due maintenance jobs on each tick when the corresponding
/// option is enabled. Big jobs (blurhash, perceptual hashes) therefore never
/// block the UI for long.
pub(crate) fn maintenance_tick(state: &mut AppState) -> Vec<UiCommand> {
    if state.import_queue.running {
        return Vec::new();
    }

    let options = state.controller.options();

    if !options.get_boolean("file_maintenance_during_active")
        && !options.get_boolean("file_maintenance_during_idle")
    {
        return Vec::new();
    }

    let (done, failed) = run_maintenance(state, 2);

    if done == 0 && failed == 0 {
        return Vec::new();
    }

    state.message = format!("file maintenance: {done} done, {failed} failed");
    vec![UiCommand::Status(render::status_json(state))]
}

pub(crate) fn run_maintenance(state: &mut AppState, limit: usize) -> (usize, usize) {
    let db = state.controller.db().db();
    let executor = crate::files::maintenance_executor::FilesMaintenanceExecutor::new(
        state.controller.files_manager(),
    );
    let manager = state.controller.maintenance();
    let mut done = 0;
    let mut failed = 0;

    for _ in 0..limit {
        let mut failure = false;
        let Some(job) = manager.run_next(|job| {
            let Ok(Some(hash_id)) = crate::db::master::find_hash_id(db, &job.hash) else {
                failure = true;
                return false;
            };

            match executor.run_job(db, hash_id, job.job_type) {
                Ok(true) => true,
                _ => {
                    failure = true;
                    false
                }
            }
        }) else {
            break;
        };

        if failure {
            failed += 1;
            // Unsupported or broken jobs are dropped instead of being retried
            // forever on every tick.
            manager.clear_jobs(&[job.hash], job.job_type);
        } else {
            done += 1;
        }
    }

    (done, failed)
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

    /// Progress-only updates for every file page; used while imports change
    /// so the loading panel can animate without a full results rebuild.
    pub(crate) fn query_progress_commands(&self) -> Vec<UiCommand> {
        self.query_ids()
            .into_iter()
            .filter_map(|id| {
                let page = self.find_query(&id)?;
                Some(UiCommand::PageProgress(
                    id,
                    render::query_progress_json(page, Some(&self.import_queue)),
                ))
            })
            .collect()
    }

    pub(crate) fn refresh_all(&mut self) {
        let controller = self.controller.clone();
        let rules = self.auto_rules.clone();
        let mut review = std::mem::take(&mut self.duplicate_review);
        let mut queued = 0usize;

        refresh_notebook(&mut self.root, &controller, &rules, &mut review, &mut queued);

        self.duplicate_review = review;

        if queued > 0 {
            let _ = auto_resolution::save_state_review(self);
        }
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
            "selection_changed" => {
                if let Some(page) = self.find_query_mut(target) {
                    let hashes: Vec<String> = serde_json::from_str(payload).unwrap_or_default();
                    page.selected = hashes
                        .iter()
                        .filter_map(|hash| query::unhex(hash))
                        .collect();
                }

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
            "notify" => vec![UiCommand::NotifyMessage(payload.to_string())],
            // Python has no subscriptions page: network -> subscriptions... opens
            // the manage subscriptions dialog.
            "manage_subscriptions" => download::manage_subscriptions(self),
            "new_subscriptions" => {
                let notebook = self.active_notebook_id();
                self.create_page(&notebook, "subscriptions")
            }
            // Python has no import folders page: file -> import folders opens
            // the edit import folders dialog.
            "manage_import_folders" => import_folders::manage(self),
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
            "import_folder_check_all" => {
                let names: Vec<String> = self
                    .import_folders
                    .iter()
                    .map(|folder| folder.name.clone())
                    .collect();
                let mut commands = Vec::new();

                for name in names {
                    commands.extend(import_folders::check_now(self, &name));
                }

                if commands.is_empty() {
                    commands.push(UiCommand::NotifyMessage(
                        "there are no import folders".to_string(),
                    ));
                }

                commands
            }
            "new_export_folders" | "export_folders" => {
                let notebook = self.active_notebook_id();
                export::create_page(self, &notebook)
            }
            "export_folder_edit" => export::edit(self, payload),
            "export_folder_save" => export::save_action(self, payload),
            "export_folder_remove" => export::remove(self, payload),
            "export_folder_toggle" => export::toggle(self, payload),
            "export_folder_run" => export::run_now(self, payload),
            "export_folder_review" => export::review(self, payload),
            "export_folder_run_all" => export::run_all(self),
            "export_folder_reveal" => export::reveal(self, payload),
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
            "command_palette" => vec![UiCommand::CommandPalette],
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
                    if page.query != payload {
                        page.query_undo.push(page.query.clone());

                        if page.query_undo.len() > 50 {
                            page.query_undo.remove(0);
                        }

                        page.query_redo.clear();
                    }

                    page.query = payload.to_string();
                    page.fixed_hashes = None;
                    page.loading = true;
                }

                search_history::record(&mut self.search_history, payload);
                let _ = search_history::persist(self);

                let mut commands = vec![UiCommand::Menus(render::menus_json(self))];

                if let Some(page) = self.find_query(target) {
                    let import = self.import_queue.clone();
                    commands.push(UiCommand::UpdatePage(
                        target.to_string(),
                        render::query_state(&controller, page, Some(&import)).to_string(),
                    ));
                    commands.push(UiCommand::Defer {
                        action: "run_query".to_string(),
                        target: target.to_string(),
                        payload: String::new(),
                        delay_ms: 1,
                    });
                }

                commands.push(self.status_command());
                commands
            }
            "run_query" => {
                let controller = self.controller.clone();
                let import = self.import_queue.clone();

                let json = {
                    let Some(page) = self.find_query_mut(target) else {
                        return vec![UiCommand::NotifyMessage(
                            "page is not a file page".to_string(),
                        )];
                    };

                    page.loading = false;
                    query::refresh_page(&controller, page);
                    render::query_state(&controller, page, Some(&import)).to_string()
                };

                vec![
                    UiCommand::UpdatePage(target.to_string(), json),
                    self.status_command(),
                ]
            }
            "or_dialog" => {
                let initial = self
                    .find_query(target)
                    .map(|page| page.query.clone())
                    .unwrap_or_default();

                vec![UiCommand::PromptOr {
                    page_id: target.to_string(),
                    initial,
                }]
            }
            "or_insert" => {
                let controller = self.controller.clone();

                if let Some(page) = self.find_query_mut(target) {
                    let mut query = page.query.trim().to_string();

                    if !query.is_empty() {
                        query.push(' ');
                    }

                    query.push_str(payload);

                    if page.query != query {
                        page.query_undo.push(page.query.clone());
                        page.query_redo.clear();
                    }

                    page.query = query;
                    page.fixed_hashes = None;
                    page.loading = true;
                } else {
                    return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                }

                let mut commands = vec![UiCommand::Menus(render::menus_json(self))];
                let import = self.import_queue.clone();

                if let Some(page) = self.find_query(target) {
                    commands.push(UiCommand::UpdatePage(
                        target.to_string(),
                        render::query_state(&controller, page, Some(&import)).to_string(),
                    ));
                    commands.push(UiCommand::Defer {
                        action: "run_query".to_string(),
                        target: target.to_string(),
                        payload: String::new(),
                        delay_ms: 1,
                    });
                }

                commands.push(self.status_command());
                commands
            }
            "notifications" => vec![UiCommand::NotificationsDialog(render::notifications_json(
                self,
            ))],
            "notifications_clear" => {
                self.notifications.clear();
                vec![
                    UiCommand::Menus(render::menus_json(self)),
                    self.status_command(),
                ]
            }
            "downloader_export" => {
                let Some(page_id) = self.active.clone().filter(|id| {
                    matches!(
                        model::find_node(&self.root, id),
                        Some(PageNode::Downloader(_))
                    )
                }) else {
                    return vec![UiCommand::NotifyMessage(
                        "select a downloader page first".to_string(),
                    )];
                };

                vec![UiCommand::ExportDownloader(page_id)]
            }
            "downloader_import" => {
                let target = self.active.clone().unwrap_or_else(|| "root".to_string());
                vec![UiCommand::ImportDownloader(target)]
            }
            "downloader_export_to" => {
                let path = PathBuf::from(payload);
                let Some(node) = model::find_node(&self.root, target) else {
                    return vec![UiCommand::NotifyMessage("page is missing".to_string())];
                };
                let PageNode::Downloader(page) = node else {
                    return vec![UiCommand::NotifyMessage(
                        "page is not a downloader".to_string(),
                    )];
                };

                let json = downloader_export::to_json(page);
                let text = match serde_json::to_string_pretty(&json) {
                    Ok(text) => text,
                    Err(error) => {
                        return vec![UiCommand::Message(
                            "export downloader".to_string(),
                            error.to_string(),
                        )];
                    }
                };

                match std::fs::write(&path, text) {
                    Ok(()) => {
                        self.message = format!("exported downloader to {}", path.display());
                        vec![
                            UiCommand::Menus(render::menus_json(self)),
                            self.status_command(),
                        ]
                    }
                    Err(error) => vec![UiCommand::Message(
                        "export downloader".to_string(),
                        format!("could not write the definition: {error}"),
                    )],
                }
            }
            "downloader_import_from" => {
                let text = match std::fs::read_to_string(payload) {
                    Ok(text) => text,
                    Err(error) => {
                        return vec![UiCommand::Message(
                            "import downloader".to_string(),
                            format!("could not read the definition: {error}"),
                        )];
                    }
                };
                let value: Value = match serde_json::from_str(&text) {
                    Ok(value) => value,
                    Err(error) => {
                        return vec![UiCommand::Message(
                            "import downloader".to_string(),
                            format!("the definition is not valid JSON: {error}"),
                        )];
                    }
                };

                let new_id = self.new_id("p");
                let page = match downloader_export::from_json(&value, &new_id) {
                    Ok(page) => page,
                    Err(error) => {
                        return vec![UiCommand::Message("import downloader".to_string(), error)];
                    }
                };

                let notebook_id = if self.find_notebook(target).is_some() {
                    target.to_string()
                } else {
                    model::parent_notebook_id(&self.root, target)
                        .unwrap_or_else(|| "root".to_string())
                };

                let Some(notebook) = self.find_notebook_mut(&notebook_id) else {
                    return vec![UiCommand::NotifyMessage(
                        "target notebook is missing".to_string(),
                    )];
                };

                notebook.add(PageNode::Downloader(page));
                notebook.current = Some(new_id.clone());
                self.select_page(&new_id);
                self.record_history(&new_id);
                self.refresh_all();
                self.message = "imported downloader definition".to_string();

                vec![
                    UiCommand::Menus(render::menus_json(self)),
                    UiCommand::BuildPages(render::pages_json(self)),
                    self.status_command(),
                ]
            }
            "viewing_stats" => {
                let hash_bytes = if payload.is_empty() {
                    let Some(page) = self.active_query() else {
                        return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                    };
                    let Some(hash) = page.focused.clone() else {
                        return vec![UiCommand::NotifyMessage("no file selected".to_string())];
                    };
                    hash
                } else {
                    let Some(hash) = query::unhex(payload) else {
                        return vec![UiCommand::NotifyMessage("invalid file hash".to_string())];
                    };
                    hash
                };

                vec![UiCommand::ViewingStatsDialog(render::viewing_stats_json(
                    &self.controller,
                    &hash_bytes,
                ))]
            }
            "undo_search" => {
                let controller = self.controller.clone();
                let page_id = self.page_id_or_active(target);

                if let Some(page) = self.find_query_mut(&page_id) {
                    let Some(previous) = page.query_undo.pop() else {
                        return vec![UiCommand::NotifyMessage("no search to undo".to_string())];
                    };

                    page.query_redo.push(page.query.clone());
                    page.query = previous;
                    page.fixed_hashes = None;
                } else {
                    return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                }

                let mut commands = vec![UiCommand::Menus(render::menus_json(self))];
                commands.extend(self.replace_query_and_refresh(&controller, &page_id));
                commands
            }
            "redo_search" => {
                let controller = self.controller.clone();
                let page_id = self.page_id_or_active(target);

                if let Some(page) = self.find_query_mut(&page_id) {
                    let Some(next) = page.query_redo.pop() else {
                        return vec![UiCommand::NotifyMessage("no search to redo".to_string())];
                    };

                    page.query_undo.push(page.query.clone());
                    page.query = next;
                    page.fixed_hashes = None;
                } else {
                    return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                }

                let mut commands = vec![UiCommand::Menus(render::menus_json(self))];
                commands.extend(self.replace_query_and_refresh(&controller, &page_id));
                commands
            }
            "apply_search_history" => {
                let controller = self.controller.clone();
                let page_id = self.page_id_or_active("");

                if let Some(page) = self.find_query_mut(&page_id) {
                    if page.query != target {
                        page.query_undo.push(page.query.clone());
                        page.query_redo.clear();
                    }

                    page.query = target.to_string();
                    page.fixed_hashes = None;
                } else {
                    return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                }

                let mut commands = vec![UiCommand::Menus(render::menus_json(self))];
                commands.extend(self.replace_query_and_refresh(&controller, &page_id));
                commands
            }
            "set_location" => {
                if let Some(page) = self.find_query_mut(target) {
                    if page.location != payload {
                        page.location = payload.to_string();
                        page.fixed_hashes = None;
                    }
                } else {
                    return vec![UiCommand::NotifyMessage(
                        "page is not a file page".to_string(),
                    )];
                }

                let controller = self.controller.clone();
                self.replace_query_and_refresh(&controller, target)
            }
            "set_tag_service" => {
                if let Some(page) = self.find_query_mut(target) {
                    if page.tag_service != payload {
                        page.tag_service = payload.to_string();
                        page.fixed_hashes = None;
                    }
                } else {
                    return vec![UiCommand::NotifyMessage(
                        "page is not a file page".to_string(),
                    )];
                }

                let controller = self.controller.clone();
                self.replace_query_and_refresh(&controller, target)
            }
            "autocomplete" => {
                let Some(page) = self.find_query(target) else {
                    return vec![];
                };

                let Some(tag_service) =
                    query::tag_service_id_for(&self.controller, &page.tag_service)
                else {
                    return vec![UiCommand::NotifyMessage("no local tag service".to_string())];
                };

                let tokens: Vec<&str> = payload.split_whitespace().collect();
                let mut token = String::new();
                let mut entries = Vec::new();

                for count in (1..=tokens.len().min(3)).rev() {
                    let candidate = tokens[tokens.len() - count..].join(" ");

                    if candidate.to_ascii_lowercase().starts_with("system:") && count > 1 {
                        continue;
                    }

                    let candidate_entries = search::autocomplete_entries(
                        self.controller.db().db(),
                        tag_service,
                        &candidate,
                        25,
                    );

                    if !candidate_entries.is_empty() {
                        token = candidate;
                        entries = candidate_entries;
                        break;
                    }
                }

                let entries: Vec<Value> = entries
                    .into_iter()
                    .map(|entry| {
                        json!({
                            "text": entry.text,
                            "category": entry.category,
                            "count": entry.count,
                        })
                    })
                    .collect();

                vec![UiCommand::Autocomplete(
                    json!({
                        "page_id": page.id,
                        "raw": token,
                        "entries": entries,
                    })
                    .to_string(),
                )]
            }
            "set_sort" => {
                let Ok(request) = serde_json::from_str::<Value>(payload) else {
                    return vec![UiCommand::NotifyMessage("invalid sort request".to_string())];
                };

                if let Some(page) = self.find_query_mut(target) {
                    if let Some(sort) = request.get("sort").and_then(Value::as_str) {
                        page.sort = sort.to_string();
                    }

                    if let Some(ascending) = request.get("ascending").and_then(Value::as_bool) {
                        page.sort_ascending = ascending;
                    }
                } else {
                    return vec![UiCommand::NotifyMessage(
                        "page is not a file page".to_string(),
                    )];
                }

                let controller = self.controller.clone();
                self.replace_query_and_refresh(&controller, target)
            }
            "favourite_save" => {
                let initial = self
                    .active_query()
                    .map(|page| {
                        if page.query.trim().is_empty() {
                            page.title.clone()
                        } else {
                            page.query.clone()
                        }
                    })
                    .unwrap_or_default();

                vec![UiCommand::PromptText {
                    action: "favourite_save_confirmed".to_string(),
                    title: "save favourite search".to_string(),
                    label: "name".to_string(),
                    initial,
                }]
            }
            "favourite_save_confirmed" => {
                let Some(page) = self.active_query() else {
                    return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                };

                let favourite = favourites::FavouriteSearch {
                    name: payload.to_string(),
                    query: page.query.clone(),
                    location: page.location.clone(),
                    tag_service: page.tag_service.clone(),
                };
                let name = favourite.name.clone();
                favourites::add_or_replace(&mut self.favourites, favourite);
                let _ = favourites::persist(self);
                self.message = format!("saved favourite search '{name}'");

                vec![
                    UiCommand::Menus(render::menus_json(self)),
                    self.status_command(),
                ]
            }
            "favourite_apply" => {
                let Some(favourite) = self
                    .favourites
                    .iter()
                    .find(|favourite| favourite.name == target)
                    .cloned()
                else {
                    return vec![UiCommand::NotifyMessage(
                        "favourite search is missing".to_string(),
                    )];
                };

                let Some(page) = self.active_query() else {
                    return vec![UiCommand::NotifyMessage("no page selected".to_string())];
                };

                let page_id = page.id.clone();
                let controller = self.controller.clone();

                if let Some(page) = self.find_query_mut(&page_id) {
                    page.query = favourite.query;
                    page.location = favourite.location;
                    page.tag_service = favourite.tag_service;
                    page.fixed_hashes = None;
                }

                self.replace_query_and_refresh(&controller, &page_id)
            }
            "favourite_delete" => vec![UiCommand::ConfirmAction {
                action: "favourite_delete_confirmed".to_string(),
                target: target.to_string(),
                title: "delete favourite search".to_string(),
                text: format!("Delete the favourite search '{target}'?"),
            }],
            "favourite_delete_confirmed" => {
                if favourites::remove(&mut self.favourites, target) {
                    let _ = favourites::persist(self);
                    self.message = format!("deleted favourite search '{target}'");
                }

                vec![
                    UiCommand::Menus(render::menus_json(self)),
                    self.status_command(),
                ]
            }
            "add_hashes_to_page" => {
                let hashes: Vec<String> = serde_json::from_str(payload).unwrap_or_default();
                let mut parsed: Vec<Vec<u8>> = hashes
                    .iter()
                    .filter_map(|hash| query::unhex(hash))
                    .collect();
                parsed.retain(|hash| !hash.is_empty());

                if parsed.is_empty() {
                    return vec![UiCommand::NotifyMessage(
                        "no files were dragged".to_string(),
                    )];
                }

                let controller = self.controller.clone();
                let import = self.import_queue.clone();
                let Some(page) = self.find_query_mut(target) else {
                    return vec![UiCommand::NotifyMessage("page is missing".to_string())];
                };

                query::append_hashes(page, parsed);

                let json = render::query_state(&controller, page, Some(&import)).to_string();

                vec![
                    UiCommand::UpdatePage(target.to_string(), json),
                    self.status_command(),
                ]
            }
            "focus_hash" => self.focus(target, payload),
            "open_viewer" => self.open_viewer(target, payload),
            "viewer_nav" => self.viewer_nav(target, payload),
            "viewer_close" => {
                if let Some(context) = self.viewers.remove(target) {
                    record_viewing(
                        &self.controller,
                        &context.hash,
                        hydrus_time::get_now_ms().saturating_sub(context.opened_at_ms),
                        false,
                    );
                }

                vec![]
            }
            "manage_tags" => self.manage_tags(target, payload, false),
            "manage_tags_active" => self.manage_tags("", "", true),
            "edit_timestamps" => self.edit_timestamps(),
            "timestamps_save" => self.save_timestamps(payload),
            "tag_add" => metadata::tag_action(self, true, payload),
            "tag_remove" => metadata::tag_action(self, false, payload),
            "copy_tags" => metadata::copy_tags(self),
            "paste_tags" => metadata::apply_tags_selection(self, payload),
            "paste_tags_prompt" | "apply_tags_selection" => vec![UiCommand::PromptText {
                action: "apply_tags_selection_apply".to_string(),
                title: "apply tags to selection".to_string(),
                label: "tags, one per line; prefix - to remove".to_string(),
                initial: String::new(),
            }],
            "apply_tags_selection_apply" => metadata::apply_tags_selection(self, payload),
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
            "archive_hashes" => self.set_inbox_many(target, payload, false),
            "inbox_hashes" => self.set_inbox_many(target, payload, true),
            "delete_hashes" => self.delete_files_many(target, payload),
            "archive_active" => self.set_inbox_active(false),
            "inbox_active" => self.set_inbox_active(true),
            "delete_files" => self.delete_files(target, payload),
            "import_files" | "review_local_imports" => local_import::open(self),
            "local_import_open_paths" => local_import::open_paths(self, payload),
            "local_import_add_paths" => local_import::add_paths(self, payload),
            "local_import_remove" => local_import::remove_paths(self, payload),
            "local_import_search_subdirectories" => {
                local_import::set_search_subdirectories(self, payload)
            }
            "local_import_delete_after_success" => {
                local_import::set_delete_after_success(self, payload)
            }
            "local_import_import" => local_import::import_now(self),
            "local_import_options" => import_options::open_caller(self, "local_import"),
            "local_import_tagging_open" => local_import::tagging_open(self),
            "local_import_tagging_save" => local_import::tagging_save(self, payload),
            "repair_file_locations" => file_locations::open(self),
            "repair_locations_set" => file_locations::set_locations(self, payload),
            "repair_locations_probe" => file_locations::probe(self, payload),
            "repair_locations_apply" => file_locations::apply(self, payload),
            "import_options" => import_options::open(self),
            "import_options_save" => import_options::save_action(self, payload),
            "import_url" => vec![UiCommand::PromptUrl],
            "download_url" => self.download_one_url(payload),
            "import_paths" => self.import_paths(payload),
            "downloader_start" => download::start(self, target, payload),
            "downloader_pause" => download::pause(self, target),
            "downloader_jobs_set" => download::jobs_set(self, target, payload),
            "downloader_set_site" => download::set_site(self, target, payload),
            "downloader_set_formula" => download::set_formula(self, target, payload),
            "watcher_add_url" => download::set_watcher_url(self, target, payload),
            "watcher_remove" => download::remove_watcher(self, target),
            "watcher_checker_options" => vec![UiCommand::NotifyMessage(
                "checker options are not implemented yet".to_string(),
            )],
            "downloader_clear" => download::clear(self, target),
            "downloader_retry" => download::retry(self, target, payload),
            "downloader_open_results" => download::open_results(self, target),
            "downloader_import_options" => download::open_import_options(self, target),
            "downloader_import_options_save" => download::save_import_options(self, payload),
            "subscription_import_options" => {
                download::open_subscription_import_options(self, payload.trim())
            }
            "subscription_import_options_save" => {
                download::save_subscription_import_options(self, payload)
            }
            "file_seed_cache" => seeds::open_file_seed_cache(self, target),
            "file_seed_cache_action" => seeds::file_seed_action(self, payload),
            "gallery_seed_log" => seeds::open_gallery_seed_log(self, target),
            "gallery_seed_log_action" => seeds::gallery_seed_action(self, payload),
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
            "subscription_edit" => download::edit_subscription_dialog(self, target, payload),
            "subscription_edit_save" => download::save_subscription_dialog(self, target, payload),
            "subscription_add" => download::add_subscription(self, target, payload),
            "subscription_duplicate" => {
                download::duplicate_subscription(self, target, payload)
            }
            "subscription_lowercase_queries" => {
                download::lowercase_subscription_queries(self, target, payload)
            }
            "subscription_scrub_delays" => {
                download::scrub_subscription_delays(self, target, payload)
            }
            "subscription_reset" => download::reset_subscription(self, target, payload),
            "subscription_import_options_clear" => {
                download::clear_subscription_import_options(self, target, payload)
            }
            "subscription_open_results" => {
                download::open_subscription_results(self, target, payload)
            }
            "duplicates_rules" => vec![UiCommand::DuplicatesRulesDialog(
                auto_resolution::dialog_json(self),
            )],
            "duplicates_merge_options_dialog" => vec![UiCommand::DuplicatesMergeOptionsDialog(
                duplicates::merge_options_json(self),
            )],
            "duplicates_rule_add" | "duplicates_rule_save" => {
                auto_resolution::save_rule(self, payload)
            }
            "duplicates_rule_remove" => auto_resolution::remove_rule(self, payload),
            "duplicates_rule_preview" => auto_resolution::preview_rule(self, payload),
            "duplicates_review" => auto_resolution::review_open(self),
            "duplicates_show_pair" => auto_resolution::show_pair_action(self, payload),
            "duplicates_merge_options" => duplicates::save_merge_options_action(self, payload),
            "duplicates_review_apply" => auto_resolution::review_apply(self, payload),
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
            "manage_services" => vec![UiCommand::ServicesDialog(services::services_json(self))],
            "api_permissions" => api_permissions::open(self),
            "api_permissions_save" => api_permissions::save(self, payload),
            "api_permissions_delete" => api_permissions::delete(self, payload),
            "service_save" => services::save_service(self, payload),
            "service_delete" => services::delete_service(self, payload),
            "login_save" => services::login_save(self, payload),
            "login_remove" => services::login_remove(self, payload),
            "admin_connect" => services::admin_connect(self, payload),
            "admin_service_action" => services::admin_service_action(self, payload),
            "admin_add_service" => services::admin_add_service(self, payload),
            "admin_edit_service" => services::admin_edit_service(self, payload),
            "admin_remove_service" => services::admin_remove_service(self, payload),
            "admin_account_action" => services::admin_account_action(self, payload),
            "account_types_open" => services::open_account_types(self, payload),
            "account_type_save" => services::save_account_type(self, payload),
            "account_type_delete" => services::delete_account_type(self, payload),
            "service_add" | "service_rename" => services::save_service(self, payload),
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
            "about" => vec![UiCommand::AboutDialog(render::about_json(self))],
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
            "tag_filter_open" => tag_filter::open(self, payload),
            "tag_filter_save" => tag_filter::save(self, payload),
            "tag_filter_test" => tag_filter::test(self, payload),
            "purge_tags" => tag_filter::purge(self, payload),
            other if other.starts_with("tag_filter:") => {
                tag_filter::open(self, other.trim_start_matches("tag_filter:"))
            }
            "pause_network" => options::network_action(self, "network_pause", ""),
            "tag_siblings" => tag_relations::show(self, tag_relations::SIBLINGS),
            "tag_parents" => tag_relations::show(self, tag_relations::PARENTS),
            "tag_relations_add" => tag_relations::action(self, payload, true),
            "tag_relations_remove" => tag_relations::action(self, payload, false),
            "orphan_files_scan" => orphans::open(self),
            "orphan_files_delete" => orphans::delete(self, payload),
            "orphan_files_move" => orphans::move_to(self, payload),
            "db_backup_open" => backup::open(self),
            "db_backup_set_path" => backup::set_path(self, payload),
            "db_backup_run" => backup::run(self),
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
            "db_analyze" => {
                let result = self
                    .controller
                    .db()
                    .db()
                    .execute_batch("PRAGMA optimize; ANALYZE;");

                match result {
                    Ok(()) => vec![UiCommand::Message(
                        "analyze database".to_string(),
                        "statistics were rebuilt and the query planner was optimized".to_string(),
                    )],
                    Err(error) => vec![UiCommand::Message(
                        "analyze database".to_string(),
                        format!("analyze failed: {error}"),
                    )],
                }
            }
            "db_vacuum_info" => {
                let db = self.controller.db().db();
                let page_count = pragma_i64(db, "page_count");
                let freelist = pragma_i64(db, "freelist_count");
                let page_size = pragma_i64(db, "page_size");
                let free_bytes = freelist.saturating_mul(page_size);
                let total_bytes = page_count.saturating_mul(page_size);

                vec![UiCommand::Message(
                    "vacuum data".to_string(),
                    format!(
                        "pages: {page_count}\nfree pages: {freelist}\npage size: {page_size} bytes\ntotal: {}\nreusable: {}",
                        human_bytes(total_bytes as u64),
                        human_bytes(free_bytes as u64)
                    ),
                )]
            }
            "db_deferred_deletes" => {
                let db = self.controller.db().db();
                let trash = query::trash_service_id(&self.controller)
                    .and_then(|service_id| {
                        files_storage::get_hash_ids(db, service_id, ContentStatus::Current).ok()
                    })
                    .map(|ids| ids.len())
                    .unwrap_or(0);

                vec![UiCommand::Message(
                    "deferred delete tables".to_string(),
                    format!("files waiting in trash: {trash}"),
                )]
            }
            "db_file_maintenance" => vec![UiCommand::Message(
                "file maintenance jobs".to_string(),
                render::maintenance_json(self),
            )],
            "db_run_file_maintenance" => {
                let (done, failed) = run_maintenance(self, 500);
                self.message = format!("file maintenance finished: {done} done, {failed} failed");
                vec![
                    UiCommand::NotifyMessage(self.message.clone()),
                    UiCommand::Message(
                        "file maintenance jobs".to_string(),
                        format!(
                            "{}\n\n{done} job(s) completed, {failed} job(s) failed.",
                            render::maintenance_json(self)
                        ),
                    ),
                    self.status_command(),
                ]
            }
            "db_fix_archive_times" => {
                let manager = self.controller.services();
                let Some(service) = manager.get_service(b"my files") else {
                    return vec![UiCommand::NotifyMessage(
                        "the 'my files' service is missing".to_string(),
                    )];
                };

                match files_timestamps::fix_missing_archive_times(
                    self.controller.db().db(),
                    service.service_id,
                    hydrus_time::get_now(),
                ) {
                    Ok(fixed) => {
                        self.message = format!("fixed archive times for {fixed} files");
                        vec![
                            UiCommand::NotifyMessage(self.message.clone()),
                            self.status_command(),
                        ]
                    }
                    Err(error) => vec![UiCommand::NotifyMessage(format!(
                        "could not fix archive times: {error}"
                    ))],
                }
            }
            "db_stats" => {
                let db = self.controller.db().db();
                let version = self.controller.db().get_version().unwrap_or(0);
                let services = self.controller.services().get_num_services();
                let files = scalar_i64(db, "SELECT COUNT(*) FROM files_info");
                let inbox = files_storage::get_num_inbox(db).unwrap_or(0);
                let trash = query::trash_service_id(&self.controller)
                    .and_then(|service_id| {
                        files_storage::get_hash_ids(db, service_id, ContentStatus::Current).ok()
                    })
                    .map(|ids| ids.len() as i64)
                    .unwrap_or(0);
                let tags = query::tag_service_id(&self.controller)
                    .and_then(|service_id| {
                        mappings::get_num_mappings(db, service_id, ContentStatus::Current).ok()
                    })
                    .unwrap_or(0);

                vec![UiCommand::Message(
                    "how boned am I?".to_string(),
                    format!(
                        "database version: {version}\nservices: {services}\nfiles with metadata: {files}\ninbox: {inbox}\ntrash: {trash}\nlocal tag mappings: {tags}"
                    ),
                )]
            }
            "pending" => vec![UiCommand::NotifyMessage(
                "there is no pending upload in this local-only build".to_string(),
            )],
            "undo" => match self.undo_closed_page() {
                Some(id) => {
                    self.message = format!("restored page '{id}'");
                    vec![
                        UiCommand::Menus(render::menus_json(self)),
                        UiCommand::BuildPages(render::pages_json(self)),
                        self.status_command(),
                    ]
                }
                None => vec![UiCommand::NotifyMessage("nothing to undo".to_string())],
            },
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
        let import = self.import_queue.clone();

        if let Some(page) = self.find_query_mut(page_id) {
            query::refresh_page(&controller, page);
            let json = render::query_state(&controller, page, Some(&import)).to_string();

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
        let import = self.import_queue.clone();
        let json = {
            let Some(page) = self.find_query_mut(page_id) else {
                return vec![UiCommand::NotifyMessage(
                    "page is not a file page".to_string(),
                )];
            };

            query::refresh_page(controller, page);
            render::query_state(controller, page, Some(&import)).to_string()
        };

        vec![
            UiCommand::UpdatePage(page_id.to_string(), json),
            self.status_command(),
        ]
    }

    pub(crate) fn refresh_commands(&mut self) -> Vec<UiCommand> {
        let controller = self.controller.clone();
        let import = self.import_queue.clone();
        let ids = self.query_ids();
        let mut commands = Vec::new();

        for id in ids {
            let json = {
                let Some(page) = self.find_query_mut(&id) else {
                    continue;
                };
                query::refresh_page(&controller, page);
                render::query_state(&controller, page, Some(&import)).to_string()
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
        } else if kind == "export_folders" {
            return export::create_page(self, notebook_id);
        } else if kind == "duplicates" {
            let id = self.new_id("p");
            page_id = id.clone();
            let mut page = DuplicatesPage::new(id);
            let (_, queued) = duplicates::refresh_for(
                &controller,
                &mut page,
                &self.auto_rules,
                &mut self.duplicate_review,
            );

            if queued > 0 {
                let _ = auto_resolution::save_state_review(self);
            }

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

        let parent_id = model::parent_notebook_id(&self.root, page_id);
        let index = parent_id.as_deref().and_then(|parent_id| {
            self.find_notebook(parent_id).and_then(|parent| {
                parent
                    .children
                    .iter()
                    .position(|child| child.id() == page_id)
            })
        });
        let closed_node = model::find_node(&self.root, page_id).cloned();

        if !self.remove_page(page_id) {
            return vec![UiCommand::NotifyMessage("page is missing".to_string())];
        }

        if let (Some(parent_id), Some(index), Some(node)) = (parent_id, index, closed_node) {
            self.push_closed_page(parent_id, index, node);
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

        let mut removed = Vec::new();
        let mut closed = Vec::new();

        {
            let Some(parent) = self.find_notebook_mut(&parent_id) else {
                return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
            };

            let mut kept = Vec::new();

            for (index, child) in parent.children.drain(..).enumerate() {
                if child.id() == page_id {
                    kept.push(child);
                } else {
                    collect_page_ids(&child, &mut removed);
                    closed.push((parent_id.clone(), index, child));
                }
            }

            parent.children = kept;
            parent.current = Some(page_id.to_string());
        }

        for (parent_id, index, node) in closed {
            self.push_closed_page(parent_id, index, node);
        }

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
            PageNode::ExportFolders(page) => page.title = name.to_string(),
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

        record_viewing(&controller, &hash, 0, true);

        self.viewers.insert(
            viewer_id.clone(),
            ViewerContext {
                page_id: page_id.to_string(),
                hash: hash.clone(),
                opened_at_ms: hydrus_time::get_now_ms(),
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

        record_viewing(
            &controller,
            &context.hash,
            hydrus_time::get_now_ms().saturating_sub(context.opened_at_ms),
            false,
        );
        record_viewing(&controller, &next, 0, true);

        self.viewers.insert(
            viewer_id.to_string(),
            ViewerContext {
                page_id: context.page_id.clone(),
                hash: next.clone(),
                opened_at_ms: hydrus_time::get_now_ms(),
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

        if active {
            let Some(page) = self.active_query() else {
                return vec![UiCommand::NotifyMessage("no page selected".to_string())];
            };

            let hashes = if page.selected.len() > 1 {
                page.selected.clone()
            } else if let Some(hash) = page.focused.clone() {
                vec![hash]
            } else {
                return vec![UiCommand::NotifyMessage("no file selected".to_string())];
            };

            return vec![UiCommand::ManageTagsDialog(
                metadata::manage_tags_json(&controller, &hashes).to_string(),
            )];
        }

        let Some(hash) = query::unhex(hash_hex) else {
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

        let hashes: Vec<Vec<u8>> = if let Some(list) = value.get("hashes").and_then(Value::as_array)
        {
            list.iter()
                .filter_map(Value::as_str)
                .filter_map(query::unhex)
                .collect()
        } else {
            value
                .get("hash")
                .and_then(Value::as_str)
                .and_then(query::unhex)
                .into_iter()
                .collect()
        };

        if hashes.is_empty() {
            return vec![UiCommand::NotifyMessage("invalid file hash".to_string())];
        }

        let Some(service_id) = value
            .get("service")
            .and_then(Value::as_str)
            .and_then(|service| metadata::tag_service_id(&self.controller, service))
            .or_else(|| query::tag_service_id(&self.controller))
        else {
            return vec![UiCommand::NotifyMessage("no local tag service".to_string())];
        };

        let add: Vec<String> = value
            .get("add")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let remove: Vec<String> = value
            .get("remove")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        let db = self.controller.db().db();
        let mut changed = 0;
        let mut found = 0;

        for hash in &hashes {
            let Some(hash_id) = query::hash_id_for(&self.controller, hash) else {
                continue;
            };

            found += 1;

            for tag in &add {
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

            for tag in &remove {
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

        if found == 0 {
            return vec![UiCommand::NotifyMessage(
                "file is not in the database".to_string(),
            )];
        }

        self.message = format!("tags updated ({changed} changes on {found} files)");

        let mut commands = if hashes.len() == 1 {
            metadata::refresh_hash(self, &hashes[0])
        } else {
            self.refresh_commands()
        };
        commands.push(self.status_command());

        commands
    }

    fn edit_timestamps(&mut self) -> Vec<UiCommand> {
        let Some(page) = self.active_query() else {
            return vec![UiCommand::NotifyMessage("no page selected".to_string())];
        };
        let Some(hash) = page.focused.clone() else {
            return vec![UiCommand::NotifyMessage("no file selected".to_string())];
        };
        let Some(hash_id) = query::hash_id_for(&self.controller, &hash) else {
            return vec![UiCommand::NotifyMessage(
                "file is not in the database".to_string(),
            )];
        };

        let db = self.controller.db().db();
        let archived = files_timestamps::get_file_timestamp(db, hash_id, TimestampType::Archived)
            .ok()
            .flatten();
        let modified =
            files_timestamps::get_file_timestamp(db, hash_id, TimestampType::ModifiedFile)
                .ok()
                .flatten();

        let json = json!({
            "hash": query::hex(&hash),
            "archived": archived,
            "modified": modified,
        })
        .to_string();

        vec![UiCommand::TimestampsDialog(json)]
    }

    fn save_timestamps(&mut self, payload: &str) -> Vec<UiCommand> {
        let Ok(value) = serde_json::from_str::<Value>(payload) else {
            return vec![UiCommand::NotifyMessage(
                "invalid timestamp payload".to_string(),
            )];
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

        let timestamp = |key: &str| {
            value.get(key).and_then(|entry| {
                if entry.is_null() {
                    Some(None)
                } else {
                    entry.as_i64().map(Some)
                }
            })
        };

        let db = self.controller.db().db();

        if let Some(archived) = timestamp("archived") {
            let _ = files_timestamps::set_file_timestamp(
                db,
                hash_id,
                TimestampType::Archived,
                archived,
            );
        }

        if let Some(modified) = timestamp("modified") {
            let _ = files_timestamps::set_file_timestamp(
                db,
                hash_id,
                TimestampType::ModifiedFile,
                modified,
            );
        }

        self.message = "timestamps updated".to_string();
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
        let _ = files_timestamps::set_file_timestamp(
            self.controller.db().db(),
            hash_id,
            TimestampType::Archived,
            if inbox {
                None
            } else {
                Some(hydrus_time::get_now())
            },
        );

        self.message = if inbox {
            "file moved to inbox".to_string()
        } else {
            "file archived".to_string()
        };

        let _ = page_id;
        self.refresh_commands()
    }

    fn set_inbox_many(&mut self, page_id: &str, payload: &str, inbox: bool) -> Vec<UiCommand> {
        let hashes: Vec<String> = serde_json::from_str(payload).unwrap_or_default();
        let mut hash_ids = Vec::new();

        for hash_hex in hashes {
            if let Some(hash) = query::unhex(&hash_hex) {
                if let Some(hash_id) = query::hash_id_for(&self.controller, &hash) {
                    hash_ids.push(hash_id);
                }
            }
        }

        if hash_ids.is_empty() {
            return vec![UiCommand::NotifyMessage("no files selected".to_string())];
        }

        let _ = files_storage::set_inbox(self.controller.db().db(), &hash_ids, inbox);

        let now = hydrus_time::get_now();
        for hash_id in &hash_ids {
            let _ = files_timestamps::set_file_timestamp(
                self.controller.db().db(),
                *hash_id,
                TimestampType::Archived,
                if inbox { None } else { Some(now) },
            );
        }

        let count = hash_ids.len();

        self.message = if inbox {
            format!("{count} files moved to inbox")
        } else {
            format!("{count} files archived")
        };

        let _ = page_id;
        self.refresh_commands()
    }

    fn delete_files_many(&mut self, page_id: &str, payload: &str) -> Vec<UiCommand> {
        let hashes: Vec<String> = serde_json::from_str(payload).unwrap_or_default();
        let mut hash_ids = Vec::new();

        for hash_hex in hashes {
            if let Some(hash) = query::unhex(&hash_hex) {
                if let Some(hash_id) = query::hash_id_for(&self.controller, &hash) {
                    hash_ids.push(hash_id);
                }
            }
        }

        if hash_ids.is_empty() {
            return vec![UiCommand::NotifyMessage("no files selected".to_string())];
        }

        let db = self.controller.db().db();
        let my_files = query::local_file_service_id(&self.controller, "my files");
        let now = hydrus_time::get_now();

        if let Some(trash) = query::trash_service_id(&self.controller) {
            let _ =
                files_storage::add_files(db, trash, ContentStatus::Current, &hash_ids, Some(now));
        }

        if let Some(service) = my_files {
            let _ = files_storage::remove_files(db, service, ContentStatus::Current, &hash_ids);
        }

        for hash_id in &hash_ids {
            let _ = files_storage::set_inbox(db, &[*hash_id], false);
        }

        self.message = format!("{} files moved to trash", hash_ids.len());
        let _ = page_id;
        self.refresh_commands()
    }

    fn set_inbox_active(&mut self, inbox: bool) -> Vec<UiCommand> {
        let Some(page) = self.active_query() else {
            return vec![UiCommand::NotifyMessage("no page selected".to_string())];
        };

        let page_id = page.id.clone();

        if page.selected.len() > 1 {
            let hashes: Vec<String> = page.selected.iter().map(|hash| query::hex(hash)).collect();
            let payload = serde_json::to_string(&hashes).unwrap_or_else(|_| "[]".to_string());

            return self.set_inbox_many(&page_id, &payload, inbox);
        }

        let Some(hash) = page.focused.clone() else {
            return vec![UiCommand::NotifyMessage("no file selected".to_string())];
        };

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
            filename_tagging: Default::default(),
            delete_after_success: false,
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
    review: &mut auto_resolution::DuplicateReviewState,
    queued: &mut usize,
) {
    for child in &mut notebook.children {
        match child {
            PageNode::Query(page) => query::refresh_page(controller, page),
            PageNode::Notebook(inner) => refresh_notebook(inner, controller, rules, review, queued),
            PageNode::Duplicates(page) => {
                *queued += duplicates::refresh_for(controller, page, rules, review).1;
            }
            PageNode::Downloader(_)
            | PageNode::Subscriptions(_)
            | PageNode::ImportFolders(_)
            | PageNode::ExportFolders(_)
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
            | PageNode::ExportFolders(_)
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

fn collect_export_folder_ids(notebook: &Notebook, ids: &mut Vec<String>) {
    for child in &notebook.children {
        match child {
            PageNode::ExportFolders(page) => ids.push(page.id.clone()),
            PageNode::Notebook(inner) => collect_export_folder_ids(inner, ids),
            _ => {}
        }
    }
}

fn record_viewing(controller: &Controller, hash: &[u8], viewtime_ms: i64, add_view: bool) {
    if !controller
        .options()
        .get_boolean("file_viewing_statistics_active")
    {
        return;
    }

    let Some(hash_id) = query::hash_id_for(controller, hash) else {
        return;
    };

    let views = i64::from(add_view);
    let now = hydrus_time::get_now_ms();

    let _ = files_viewing_stats::add_viewing_stats(
        controller.db().db(),
        hash_id,
        0,
        views,
        viewtime_ms.max(0),
        Some(now),
    );
}

fn pragma_i64(db: &hydrus_core::hydrus_db_base::DBBase, name: &str) -> i64 {
    db.query_one(&format!("PRAGMA {name}"), &[], |row| row.get(0))
        .ok()
        .flatten()
        .unwrap_or(0)
}

fn scalar_i64(db: &hydrus_core::hydrus_db_base::DBBase, query: &str) -> i64 {
    db.query_one(query, &[], |row| row.get(0))
        .ok()
        .flatten()
        .unwrap_or(0)
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }

    format!("{value:.1} {}", UNITS[unit])
}

fn placeholder_for(kind: &str) -> (String, String) {
    (
        kind.to_string(),
        format!("Unknown page type '{kind}'.\n\nThe page was kept so it can be renamed or closed."),
    )
}
