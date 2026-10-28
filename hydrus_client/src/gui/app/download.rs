//! Downloader pages and their background jobs.
//!
//! The page model and widgets mirror Hydrus' URL downloader, gallery, watcher
//! and simple downloader pages. Network work happens in [`super::worker`]; this
//! module only builds jobs and folds progress events back into the page state.

use hydrus_core::hydrus_time;
use serde_json::{Value, json};

use crate::gui::app::model::{
    DownloadItem, DownloadItemStatus, DownloaderKind, PageNode, QueryPage, SubscriptionEntry,
};
use crate::gui::app::worker::{AsyncEvent, AsyncJob, gallery_url};
use crate::gui::app::{AppState, UiCommand, render};

pub fn start(state: &mut AppState, page_id: &str, payload: &str) -> Vec<UiCommand> {
    let value: Value = serde_json::from_str(payload).unwrap_or_else(|_| json!({}));
    let controller = state.controller.clone();

    let Some(page) = state.find_downloader_mut(page_id) else {
        return vec![UiCommand::NotifyMessage(
            "no downloader page selected".to_string(),
        )];
    };

    if page.running {
        return vec![UiCommand::NotifyMessage(
            "this page is already working".to_string(),
        )];
    }

    let input = value
        .get("input")
        .and_then(Value::as_str)
        .unwrap_or(&page.input)
        .to_string();
    let url = value
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or(&page.url)
        .to_string();
    let query = value
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or(&page.query)
        .to_string();
    let filter = value
        .get("filter")
        .and_then(Value::as_str)
        .unwrap_or(&page.filter)
        .to_string();
    let site = value
        .get("site")
        .and_then(Value::as_str)
        .unwrap_or(&page.site)
        .to_string();
    let subject = value
        .get("subject")
        .and_then(Value::as_str)
        .unwrap_or(&page.subject)
        .to_string();
    let auto = value
        .get("auto")
        .and_then(Value::as_bool)
        .unwrap_or(page.auto_watch);
    let period = value
        .get("period")
        .and_then(Value::as_i64)
        .unwrap_or(page.check_period)
        .max(60);

    page.input = input.clone();
    page.url = url.clone();
    page.query = query.clone();
    page.site = site.clone();
    page.filter = filter.clone();
    page.subject = subject;
    page.auto_watch = auto;
    page.check_period = period;
    page.imported = 0;
    page.failed = 0;
    page.results.clear();
    page.log.clear();

    let now = hydrus_time::get_now();
    page.last_check = now;

    let kind = page.kind;

    let (urls, category) = match kind {
        DownloaderKind::Urls => {
            let urls = normalise_urls(&controller, lines(&input));

            if urls.is_empty() {
                page.note("no urls to download");
                return update_commands(state, page_id);
            }

            page.queue = urls.iter().cloned().map(DownloadItem::new).collect();
            page.note(format!("queued {} urls", urls.len()));
            (urls, "url")
        }
        DownloaderKind::Simple => {
            let urls = normalise_urls(&controller, lines(&input));

            if urls.is_empty() {
                page.note("no page urls to download");
                return update_commands(state, page_id);
            }

            page.queue.clear();
            page.note(format!("queued {} pages", urls.len()));
            (urls, "page")
        }
        DownloaderKind::Gallery => {
            let resolved = gallery_url(&url, &query);

            if resolved.is_empty() {
                page.note("no gallery url set");
                return update_commands(state, page_id);
            }

            let resolved = controller
                .domain_manager()
                .read()
                .ok()
                .and_then(|manager| manager.normalise_url(&resolved))
                .unwrap_or(resolved);

            page.queue.clear();

            let file_class =
                controller.domain_manager().read().ok().and_then(|manager| {
                    manager
                        .get_url_class_for_url(&resolved)
                        .map(|class| class.url_type)
                }) == Some(crate::networking::client_networking_url_class::URL_TYPE_FILE);

            if file_class {
                page.note(format!("gallery url is a file url: {resolved}"));
                (vec![resolved], "url")
            } else {
                page.note(format!("starting gallery {resolved}"));
                (vec![resolved], "page")
            }
        }
        DownloaderKind::Watcher => {
            if url.is_empty() {
                page.note("no watched url set");
                return update_commands(state, page_id);
            }

            page.queue.clear();
            page.note(if auto {
                format!("watching {url}")
            } else {
                format!("checking {url}")
            });
            (vec![url], "page")
        }
    };

    page.running = true;
    page.paused = false;

    let job_id = page_id.to_string();
    let job = if category == "url" {
        AsyncJob::DownloadUrls { job_id, urls }
    } else {
        AsyncJob::ProcessPages {
            job_id,
            urls,
            filter,
            site: site.clone(),
        }
    };

    if let Err(error) = state.runtime.submit(job) {
        if let Some(page) = state.find_downloader_mut(page_id) {
            page.running = false;
            page.note(error);
        }
    }

    update_commands(state, page_id)
}

pub fn pause(state: &mut AppState, page_id: &str) -> Vec<UiCommand> {
    if let Some(page) = state.find_downloader_mut(page_id) {
        page.paused = !page.paused;
        page.note(if page.paused {
            "paused; the current file will finish".to_string()
        } else {
            "resumed".to_string()
        });
    }

    update_commands(state, page_id)
}

pub fn clear(state: &mut AppState, page_id: &str) -> Vec<UiCommand> {
    if let Some(page) = state.find_downloader_mut(page_id) {
        page.clear_finished();
        page.status = format!("{} items in the queue", page.queue.len());
    }

    update_commands(state, page_id)
}

pub fn retry(state: &mut AppState, page_id: &str, url: &str) -> Vec<UiCommand> {
    let Some(page) = state.find_downloader_mut(page_id) else {
        return vec![];
    };

    let Some(item) = page.item_mut(url) else {
        return vec![UiCommand::NotifyMessage(
            "download item is missing".to_string(),
        )];
    };

    if matches!(
        item.status,
        DownloadItemStatus::Pending | DownloadItemStatus::Working
    ) {
        return vec![];
    }

    item.status = DownloadItemStatus::Pending;
    item.note.clear();
    page.running = true;
    page.note(format!("retrying {url}"));

    let job = AsyncJob::DownloadUrls {
        job_id: page_id.to_string(),
        urls: vec![url.to_string()],
    };

    if let Err(error) = state.runtime.submit(job) {
        if let Some(page) = state.find_downloader_mut(page_id) {
            page.running = false;
            page.note(error);
        }
    }

    update_commands(state, page_id)
}

pub fn open_results(state: &mut AppState, page_id: &str) -> Vec<UiCommand> {
    let Some(page) = state.find_downloader(&page_id) else {
        return vec![UiCommand::NotifyMessage(
            "no downloader page selected".to_string(),
        )];
    };

    let hashes = page.results.clone();
    let title = format!("{} files", page.title);

    if hashes.is_empty() {
        return vec![UiCommand::NotifyMessage(
            "this page has not imported any files yet".to_string(),
        )];
    }

    open_hashes_page(state, &title, hashes)
}

pub fn open_subscription_results(
    state: &mut AppState,
    page_id: &str,
    name: &str,
) -> Vec<UiCommand> {
    let Some(entry) = state
        .subscriptions
        .iter()
        .find(|subscription| subscription.name == name)
    else {
        return vec![UiCommand::NotifyMessage(
            "subscription is missing".to_string(),
        )];
    };

    let hashes = entry.results.clone();
    let title = format!("{} files", entry.name);

    if hashes.is_empty() {
        return vec![UiCommand::NotifyMessage(
            "this subscription has not imported any files yet".to_string(),
        )];
    }

    let _ = page_id;
    open_hashes_page(state, &title, hashes)
}

pub(crate) fn open_hashes_page(
    state: &mut AppState,
    title: &str,
    hashes: Vec<Vec<u8>>,
) -> Vec<UiCommand> {
    let id = state.new_id("p");
    let mut query = QueryPage::new(id.clone(), title, "my files");
    query.fixed_hashes = Some(hashes);

    let notebook_id = state.active_notebook_id();

    let Some(notebook) = state.find_notebook_mut(&notebook_id) else {
        return vec![UiCommand::NotifyMessage("notebook is missing".to_string())];
    };

    notebook.add(PageNode::Query(query));
    state.active = Some(id.clone());

    let controller = state.controller.clone();

    if let Some(page) = state.find_query_mut(&id) {
        crate::gui::app::query::refresh_page(&controller, page);
    }

    vec![
        UiCommand::BuildPages(render::pages_json(state)),
        UiCommand::Status(render::status_json(state)),
    ]
}

pub fn start_subscription(state: &mut AppState, page_id: &str, name: &str) -> Vec<UiCommand> {
    let now = hydrus_time::get_now();

    let Some(entry) = state
        .subscriptions
        .iter_mut()
        .find(|subscription| subscription.name == name)
    else {
        return vec![UiCommand::NotifyMessage(
            "subscription is missing".to_string(),
        )];
    };

    if entry.running {
        return vec![UiCommand::NotifyMessage(
            "this subscription is already running".to_string(),
        )];
    }

    let url = gallery_url(&entry.url, &entry.query);

    if url.is_empty() {
        return vec![UiCommand::NotifyMessage(
            "subscription has no gallery url".to_string(),
        )];
    }

    entry.running = true;
    entry.last_check = now;
    entry.status = "checking".to_string();
    entry.failed = 0;

    let job = AsyncJob::ProcessPages {
        job_id: format!("sub:{}", entry.name),
        urls: vec![url],
        filter: entry.filter.clone(),
        site: entry.site.clone(),
    };

    if let Err(error) = state.runtime.submit(job) {
        if let Some(entry) = state
            .subscriptions
            .iter_mut()
            .find(|subscription| subscription.name == name)
        {
            entry.running = false;
            entry.status = error;
        }
    }

    subscriptions_commands(state, page_id)
}

pub fn toggle_subscription(state: &mut AppState, page_id: &str, name: &str) -> Vec<UiCommand> {
    if let Some(entry) = state
        .subscriptions
        .iter_mut()
        .find(|subscription| subscription.name == name)
    {
        entry.paused = !entry.paused;
        entry.status = if entry.paused {
            "paused".to_string()
        } else {
            "ready".to_string()
        };
    }

    subscriptions_commands(state, page_id)
}

pub fn delete_subscription(state: &mut AppState, page_id: &str, name: &str) -> Vec<UiCommand> {
    state
        .subscriptions
        .retain(|subscription| subscription.name != name);

    subscriptions_commands(state, page_id)
}

pub fn add_subscription(state: &mut AppState, page_id: &str, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return vec![UiCommand::NotifyMessage(
            "invalid subscription payload".to_string(),
        )];
    };

    let url = value
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();

    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return vec![UiCommand::NotifyMessage(
            "the subscription url must start with http:// or https://".to_string(),
        )];
    }

    let name = value
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| subscription_name_from_url(&url));

    let query = value
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let filter = value
        .get("filter")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let site = value
        .get("site")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let default_period = state
        .controller
        .options()
        .get_integer("subscription_check_period");
    let default_period = if default_period >= 60 {
        default_period
    } else {
        86_400
    };
    let period = value
        .get("period")
        .and_then(Value::as_i64)
        .unwrap_or(default_period)
        .max(60);

    let mut entry = SubscriptionEntry::new(name.clone(), url);
    entry.query = query;
    entry.site = site;
    entry.filter = filter;
    entry.period = period;

    state
        .subscriptions
        .retain(|subscription| subscription.name != name);
    state.subscriptions.push(entry);

    subscriptions_commands(state, page_id)
}

pub fn tick(state: &mut AppState) -> Vec<UiCommand> {
    let now = hydrus_time::get_now();
    let mut due_watchers: Vec<(String, String, String)> = Vec::new();
    let mut due_subscriptions: Vec<String> = Vec::new();

    collect_due_watchers(state, now, &mut due_watchers);
    collect_due_subscriptions(state, now, &mut due_subscriptions);

    for name in due_subscriptions {
        start_subscription(state, "", &name);
    }

    for (page_id, url, filter) in due_watchers {
        let job = AsyncJob::ProcessPages {
            job_id: page_id.clone(),
            urls: vec![url.clone()],
            filter,
            site: String::new(),
        };

        if state.runtime.submit(job).is_ok() {
            if let Some(page) = state.find_downloader_mut(&page_id) {
                page.running = true;
                page.last_check = now;
                page.note(format!("automatic watcher check: {url}"));
            }
        }
    }

    let events = state.runtime.drain();
    let mut commands = super::import_folders::tick(state);
    commands.extend(super::export::tick(state));

    for event in events {
        commands.extend(handle_event(state, event));
    }

    commands
}

fn collect_due_watchers(state: &AppState, now: i64, due: &mut Vec<(String, String, String)>) {
    fn visit(notebook: &super::model::Notebook, now: i64, due: &mut Vec<(String, String, String)>) {
        for child in &notebook.children {
            match child {
                PageNode::Downloader(page) => {
                    if page.kind == DownloaderKind::Watcher
                        && page.auto_watch
                        && !page.paused
                        && !page.running
                        && !page.url.is_empty()
                        && now >= page.last_check + page.check_period.max(60)
                    {
                        due.push((page.id.clone(), page.url.clone(), page.filter.clone()));
                    }
                }
                PageNode::Notebook(inner) => visit(inner, now, due),
                _ => {}
            }
        }
    }

    visit(&state.root, now, due);
}

fn collect_due_subscriptions(state: &AppState, now: i64, due: &mut Vec<String>) {
    for entry in &state.subscriptions {
        if entry.is_due(now) {
            due.push(entry.name.clone());
        }
    }
}

fn handle_event(state: &mut AppState, event: AsyncEvent) -> Vec<UiCommand> {
    match event {
        AsyncEvent::ImportProgress {
            done,
            total,
            imported,
            failed,
            message,
        } => {
            state.import_queue.running = true;
            state.import_queue.done = done;
            state.import_queue.total = total;
            state.import_queue.imported = imported;
            state.import_queue.failed = failed;
            state.import_queue.message = message.clone();
            state.message = message;

            let mut commands = state.query_progress_commands();
            commands.push(UiCommand::Status(render::status_json(state)));
            commands
        }
        AsyncEvent::ImportFinished {
            imported,
            skipped,
            errors,
            hashes,
            folder,
            results,
        } => {
            state.import_queue.running = false;
            state.import_queue.done = state.import_queue.total;
            state.import_queue.imported = imported;
            state.import_queue.failed = skipped;
            state.import_queue.message = if errors.is_empty() {
                format!("imported {imported} files")
            } else {
                format!("imported {imported} files; {} errors", errors.len())
            };
            state.message = state.import_queue.message.clone();

            let maintenance = state.controller.maintenance();

            for hash in &hashes {
                maintenance.schedule_job_now(
                    hash.clone(),
                    crate::files::maintenance::REGENERATE_FILE_DATA_JOB_FILE_MODIFIED_TIMESTAMP,
                );
                maintenance.schedule_job_now(
                    hash.clone(),
                    crate::files::maintenance::REGENERATE_FILE_DATA_JOB_FILE_HAS_TRANSPARENCY,
                );
            }

            let mut commands = Vec::new();

            if state.query_ids().is_empty() && !hashes.is_empty() {
                let id = state.new_id("p");
                let mut page = QueryPage::new(id.clone(), "files", "my files");
                page.fixed_hashes = Some(hashes);
                state.root.add(PageNode::Query(page));
                state.active = Some(id);
                let controller = state.controller.clone();

                if let Some(page) =
                    state.find_query_mut(state.active.clone().unwrap_or_default().as_str())
                {
                    crate::gui::app::query::refresh_page(&controller, page);
                }

                commands.push(UiCommand::BuildPages(render::pages_json(state)));
            } else if !hashes.is_empty() {
                commands.extend(state.refresh_commands());
            }

            if let Some(folder) = folder {
                commands.extend(super::import_folders::apply_post_import(
                    state, &folder, &results,
                ));
            }

            commands.extend(state.query_progress_commands());
            commands.push(UiCommand::Status(render::status_json(state)));
            commands
        }
        AsyncEvent::ExportFinished {
            name,
            result,
            checked_at,
        } => super::export::apply_result(state, &name, result, checked_at),
        AsyncEvent::Started { job_id, total } => {
            if let Some(name) = job_id.strip_prefix("sub:") {
                if let Some(entry) = state
                    .subscriptions
                    .iter_mut()
                    .find(|subscription| subscription.name == name)
                {
                    entry.status = format!("working on {total} files");
                }

                return subscriptions_commands(state, "");
            }

            if let Some(page) = state.find_downloader_mut(&job_id) {
                page.status = if total == 0 {
                    "no files found".to_string()
                } else {
                    format!("downloading {total} files")
                };
            }

            downloader_commands(state, &job_id)
        }
        AsyncEvent::Discovered { job_id, urls } => {
            if let Some(name) = job_id.strip_prefix("sub:") {
                if let Some(entry) = state
                    .subscriptions
                    .iter_mut()
                    .find(|subscription| subscription.name == name)
                {
                    entry.status = format!("found {} files", urls.len());
                }
            } else if let Some(page) = state.find_downloader_mut(&job_id) {
                for url in &urls {
                    if page.queue.iter().all(|item| &item.url != url) {
                        page.queue.push(DownloadItem::new(url.clone()));
                    }
                }

                page.note(format!("found {} files", urls.len()));
            }

            downloader_commands(state, &job_id)
        }
        AsyncEvent::ItemStarted { job_id, url } => {
            if let Some(name) = job_id.strip_prefix("sub:") {
                if let Some(entry) = state
                    .subscriptions
                    .iter_mut()
                    .find(|subscription| subscription.name == name)
                {
                    entry.status = format!("downloading {url}");
                }

                return subscriptions_commands(state, "");
            }

            if let Some(page) = state.find_downloader_mut(&job_id) {
                if let Some(item) = page.item_mut(&url) {
                    item.status = DownloadItemStatus::Working;
                    item.note = "downloading".to_string();
                }
            }

            downloader_commands(state, &job_id)
        }
        AsyncEvent::Message { job_id, message } => {
            if let Some(page) = state.find_downloader_mut(&job_id) {
                page.note(message);
            }

            downloader_commands(state, &job_id)
        }
        AsyncEvent::Item {
            job_id,
            url,
            ok,
            note,
            hash,
        } => {
            if job_id.starts_with("sub:") {
                if let Some(name) = job_id.strip_prefix("sub:") {
                    if let Some(entry) = state
                        .subscriptions
                        .iter_mut()
                        .find(|subscription| subscription.name == name)
                    {
                        if ok {
                            entry.imported += 1;

                            if let Some(hash) = hash {
                                entry.results.push(hash);
                            }
                        } else {
                            entry.failed += 1;
                        }

                        entry.status = note;
                    }
                }

                return subscriptions_commands(state, "");
            }

            if let Some(page) = state.find_downloader_mut(&job_id) {
                if let Some(item) = page.item_mut(&url) {
                    item.status = if ok {
                        DownloadItemStatus::Done
                    } else {
                        DownloadItemStatus::Error
                    };
                    item.note = note;
                } else {
                    let mut item = DownloadItem::new(url.clone());
                    item.status = if ok {
                        DownloadItemStatus::Done
                    } else {
                        DownloadItemStatus::Error
                    };
                    item.note = note;
                    page.queue.push(item);
                }

                if ok {
                    page.imported += 1;

                    if let Some(hash) = hash {
                        if !page.results.contains(&hash) {
                            page.results.push(hash);
                        }
                    }
                } else {
                    page.failed += 1;
                }
            }

            downloader_commands(state, &job_id)
        }
        AsyncEvent::Finished {
            job_id,
            imported,
            failed,
            message,
            hashes,
        } => {
            let mut commands = Vec::new();

            if let Some(name) = job_id.strip_prefix("sub:") {
                if let Some(entry) = state
                    .subscriptions
                    .iter_mut()
                    .find(|subscription| subscription.name == name)
                {
                    entry.running = false;
                    entry.imported = imported;
                    entry.failed = failed;
                    entry.status = message;
                    entry.results = hashes;
                }

                commands.extend(subscriptions_commands(state, ""));
            } else if let Some(page) = state.find_downloader_mut(&job_id) {
                page.running = false;
                page.imported = imported;
                page.failed = failed;
                page.results = hashes;
                page.note(message);

                if page.kind == DownloaderKind::Watcher && page.auto_watch {
                    page.status = format!("watching; next check in {} seconds", page.check_period);
                }

                commands.extend(downloader_commands(state, &job_id));
            }

            if imported > 0 {
                commands.extend(state.refresh_commands());
            }

            commands.push(UiCommand::Status(render::status_json(state)));
            commands
        }
    }
}

fn downloader_commands(state: &AppState, page_id: &str) -> Vec<UiCommand> {
    if page_id.starts_with("sub:") {
        return subscriptions_commands(state, "");
    }

    let Some(page) = state.find_downloader(page_id) else {
        return vec![];
    };

    vec![UiCommand::UpdateDownloader(
        page_id.to_string(),
        render::downloader_json(&state.controller, page),
    )]
}

fn update_commands(state: &AppState, page_id: &str) -> Vec<UiCommand> {
    let mut commands = downloader_commands(state, page_id);
    commands.push(UiCommand::Status(render::status_json(state)));
    commands
}

fn subscriptions_commands(state: &AppState, _page_id: &str) -> Vec<UiCommand> {
    let mut commands = Vec::new();

    for id in subscriptions_page_ids(&state.root) {
        commands.push(UiCommand::UpdateSubscriptions(
            id.clone(),
            render::subscriptions_json(state, &id),
        ));
    }

    commands.push(UiCommand::Status(render::status_json(state)));
    commands
}

fn subscriptions_page_ids(notebook: &super::model::Notebook) -> Vec<String> {
    let mut ids = Vec::new();

    for child in &notebook.children {
        match child {
            PageNode::Subscriptions(page) => ids.push(page.id.clone()),
            PageNode::Notebook(inner) => ids.extend(subscriptions_page_ids(inner)),
            _ => {}
        }
    }

    ids
}

fn normalise_urls(controller: &crate::controller::Controller, urls: Vec<String>) -> Vec<String> {
    let manager = match controller.domain_manager().read() {
        Ok(manager) => manager,
        Err(_) => return urls,
    };

    let mut normalised = Vec::with_capacity(urls.len());

    for url in urls {
        let url = manager.normalise_url(&url).unwrap_or(url);

        if !normalised.contains(&url) {
            normalised.push(url);
        }
    }

    normalised
}

fn lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| line.starts_with("http://") || line.starts_with("https://"))
        .map(str::to_string)
        .collect()
}

fn subscription_name_from_url(url: &str) -> String {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_else(|| "subscription".to_string())
}
