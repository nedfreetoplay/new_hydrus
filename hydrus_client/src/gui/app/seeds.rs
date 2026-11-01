//! File seed cache and gallery seed log dialogs.
//!
//! Python shows these as import-pane side panels. In the Rust port they are
//! dialogs over the live page state: file seeds come from `DownloaderPage.queue`
//! and gallery seeds from `DownloaderPage.gallery_seeds`. Both stay in the GUI
//! session, so the logs survive a restart.

use serde_json::Value;

use crate::gui::qt::api::ffi::{SeedCacheState, SeedFilter, SeedRow};

use crate::gui::app::model::{DownloadItemStatus, DownloaderKind, DownloaderPage, PageNode};
use crate::gui::app::worker::AsyncJob;
use crate::gui::app::{render, AppState, UiCommand};

fn find_page<'a>(state: &'a AppState, page_id: &str) -> Option<&'a DownloaderPage> {
    fn visit<'a>(
        notebook: &'a crate::gui::app::model::Notebook,
        id: &str,
    ) -> Option<&'a DownloaderPage> {
        for child in &notebook.children {
            match child {
                PageNode::Downloader(page) if page.id == id => return Some(page),
                PageNode::Notebook(inner) => {
                    if let Some(page) = visit(inner, id) {
                        return Some(page);
                    }
                }
                _ => {}
            }
        }

        None
    }

    visit(&state.root, page_id)
}

fn seed_counts(seeds: &[(&str, &str)]) -> (usize, usize, usize) {
    let done = seeds
        .iter()
        .filter(|(status, _)| *status == "done" || *status == "success")
        .count();
    let failed = seeds
        .iter()
        .filter(|(status, _)| *status == "error" || *status == "failed")
        .count();
    let waiting = seeds
        .iter()
        .filter(|(status, _)| *status == "pending" || *status == "working")
        .count();

    (done, failed, waiting)
}

fn seed_filters(entries: &[(&str, &str)]) -> Vec<SeedFilter> {
    entries
        .iter()
        .map(|(key, label)| SeedFilter {
            key: key.to_string(),
            label: label.to_string(),
        })
        .collect()
}

/// Typed file seed cache contents (WS-15.2.3c).
pub fn file_seed_cache_state(state: &AppState, page_id: &str) -> SeedCacheState {
    let Some(page) = find_page(state, page_id) else {
        return SeedCacheState::default();
    };

    let seeds: Vec<SeedRow> = page
        .queue
        .iter()
        .map(|item| SeedRow {
            status: item.status.as_str().to_string(),
            url: item.url.clone(),
            note: item.note.clone(),
            created: item.created,
            modified: item.modified,
        })
        .collect();
    let statuses: Vec<(&str, &str)> = page
        .queue
        .iter()
        .map(|item| (item.status.as_str(), item.url.as_str()))
        .collect();
    let (done, failed, waiting) = seed_counts(&statuses);

    SeedCacheState {
        page: page.id.clone(),
        subtitle: format!(
            "{} file seed(s): {} done, {} failed, {} waiting",
            page.queue.len(),
            done,
            failed,
            waiting,
        ),
        failed: failed as i64,
        status_filters: seed_filters(&[
            ("all", "all"),
            ("pending", "pending"),
            ("working", "working"),
            ("done", "done"),
            ("error", "failed"),
        ]),
        seeds,
    }
}

/// Typed gallery seed log contents (WS-15.2.3c).
pub fn gallery_seed_log_state(state: &AppState, page_id: &str) -> SeedCacheState {
    let Some(page) = find_page(state, page_id) else {
        return SeedCacheState::default();
    };

    let seeds: Vec<SeedRow> = page
        .gallery_seeds
        .iter()
        .map(|seed| SeedRow {
            status: seed.status.clone(),
            url: seed.url.clone(),
            note: seed.note.clone(),
            created: seed.created,
            modified: seed.modified,
        })
        .collect();
    let statuses: Vec<(&str, &str)> = page
        .gallery_seeds
        .iter()
        .map(|seed| (seed.status.as_str(), seed.url.as_str()))
        .collect();
    let (done, failed, waiting) = seed_counts(&statuses);

    SeedCacheState {
        page: page.id.clone(),
        subtitle: format!(
            "{} gallery seed(s): {} succeeded, {} failed, {} pending",
            page.gallery_seeds.len(),
            done,
            failed,
            waiting,
        ),
        failed: failed as i64,
        status_filters: seed_filters(&[
            ("all", "all"),
            ("pending", "pending"),
            ("working", "working"),
            ("success", "succeeded"),
            ("failed", "failed"),
        ]),
        seeds,
    }
}

pub fn open_file_seed_cache(state: &AppState, page_id: &str) -> Vec<UiCommand> {
    if find_page(state, page_id).is_none() {
        return vec![UiCommand::NotifyMessage(
            "no downloader page selected".to_string(),
        )];
    }

    vec![UiCommand::FileSeedCacheDialog(file_seed_cache_state(
        state, page_id,
    ))]
}

pub fn open_gallery_seed_log(state: &AppState, page_id: &str) -> Vec<UiCommand> {
    let Some(page) = find_page(state, page_id) else {
        return vec![UiCommand::NotifyMessage(
            "no gallery page selected".to_string(),
        )];
    };

    if !matches!(page.kind, DownloaderKind::Gallery | DownloaderKind::Watcher) {
        return vec![UiCommand::NotifyMessage(
            "this page has no gallery seed log".to_string(),
        )];
    }

    vec![UiCommand::GallerySeedLogDialog(gallery_seed_log_state(
        state, page_id,
    ))]
}

fn selected_urls(value: &Value) -> Vec<String> {
    value
        .get("urls")
        .and_then(Value::as_array)
        .map(|urls| {
            urls.iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|url| !url.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub fn file_seed_action(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return vec![UiCommand::NotifyMessage(
            "invalid seed cache payload".to_string(),
        )];
    };

    let page_id = value
        .get("page")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let action = value
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let urls = selected_urls(&value);

    let mut retried: Vec<String> = Vec::new();
    let message;
    let running;

    {
        let Some(page) = state.find_downloader_mut(&page_id) else {
            return vec![UiCommand::NotifyMessage(
                "the downloader page is gone".to_string(),
            )];
        };

        running = page.running;

        match action {
            "remove" => {
                let before = page.queue.len();
                page.queue.retain(|item| !urls.contains(&item.url));
                message = format!("removed {} seed(s)", before - page.queue.len());
            }
            "clear_finished" => {
                let before = page.queue.len();
                page.clear_finished();
                message = format!("removed {} finished seed(s)", before - page.queue.len());
            }
            "clear" => {
                page.queue.clear();
                message = "seed cache cleared".to_string();
            }
            "retry" => {
                if running {
                    message = "this page is already working".to_string();
                } else {
                    for item in &mut page.queue {
                        let should_retry = if urls.is_empty() {
                            item.status == DownloadItemStatus::Error
                        } else {
                            urls.contains(&item.url)
                        };

                        if should_retry {
                            item.retry();
                            retried.push(item.url.clone());
                        }
                    }

                    message = format!("retrying {} seed(s)", retried.len());
                }
            }
            _ => {
                return vec![UiCommand::NotifyMessage(format!(
                    "unknown seed cache action '{action}'"
                ))];
            }
        }
    }

    if !retried.is_empty() {
        if running {
            return vec![UiCommand::NotifyMessage(
                "this page is already working".to_string(),
            )];
        }

        let job = AsyncJob::DownloadUrls {
            job_id: page_id.clone(),
            urls: retried.clone(),
            import_options: super::download::downloader_import_options(state, &page_id),
        };

        match state.runtime.submit(job) {
            Ok(()) => {
                if let Some(page) = state.find_downloader_mut(&page_id) {
                    page.running = true;
                    page.note(format!("retrying {} seed(s)", retried.len()));
                }
            }
            Err(error) => {
                return vec![UiCommand::NotifyMessage(error)];
            }
        }
    } else if let Some(page) = state.find_downloader_mut(&page_id) {
        page.note(message.clone());
    }

    let mut commands = vec![
        UiCommand::FileSeedCacheDialog(file_seed_cache_state(state, &page_id)),
        UiCommand::NotifyMessage(message),
    ];

    if let Some(page) = find_page(state, &page_id) {
        commands.push(UiCommand::UpdateDownloader(render::downloader_page_state(
            &state.controller,
            page,
        )));
    }

    commands
}

pub fn gallery_seed_action(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return vec![UiCommand::NotifyMessage(
            "invalid gallery seed log payload".to_string(),
        )];
    };

    let page_id = value
        .get("page")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let action = value
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let urls = selected_urls(&value);

    let mut retried: Vec<String> = Vec::new();
    let message;
    let running;
    let filter;
    let site;

    {
        let Some(page) = find_page(state, &page_id) else {
            return vec![UiCommand::NotifyMessage(
                "the gallery page is gone".to_string(),
            )];
        };

        running = page.running;
        filter = page.filter.clone();
        site = page.site.clone();
    }

    {
        let Some(page) = state.find_downloader_mut(&page_id) else {
            return vec![UiCommand::NotifyMessage(
                "the gallery page is gone".to_string(),
            )];
        };

        match action {
            "remove" => {
                let before = page.gallery_seeds.len();
                page.gallery_seeds.retain(|seed| !urls.contains(&seed.url));
                message = format!(
                    "removed {} gallery seed(s)",
                    before - page.gallery_seeds.len()
                );
            }
            "clear_finished" => {
                let before = page.gallery_seeds.len();
                page.clear_finished_gallery_seeds();
                message = format!(
                    "removed {} finished gallery seed(s)",
                    before - page.gallery_seeds.len()
                );
            }
            "clear" => {
                page.gallery_seeds.clear();
                message = "gallery seed log cleared".to_string();
            }
            "retry" => {
                if running {
                    message = "this page is already working".to_string();
                } else {
                    for seed in &mut page.gallery_seeds {
                        let failed = seed.status == "failed";

                        if (urls.is_empty() && failed)
                            || (!urls.is_empty() && urls.contains(&seed.url))
                        {
                            seed.mark("pending", "waiting");
                            retried.push(seed.url.clone());
                        }
                    }

                    message = format!("retrying {} gallery seed(s)", retried.len());
                }
            }
            _ => {
                return vec![UiCommand::NotifyMessage(format!(
                    "unknown gallery seed log action '{action}'"
                ))];
            }
        }
    }

    if !retried.is_empty() {
        if running {
            return vec![UiCommand::NotifyMessage(
                "this page is already working".to_string(),
            )];
        }

        let job = AsyncJob::ProcessPages {
            job_id: page_id.clone(),
            urls: retried.clone(),
            filter,
            site,
            import_options: super::download::downloader_import_options(state, &page_id),
        };

        match state.runtime.submit(job) {
            Ok(()) => {
                if let Some(page) = state.find_downloader_mut(&page_id) {
                    page.running = true;
                    page.note(format!("retrying {} gallery seed(s)", retried.len()));
                }
            }
            Err(error) => {
                return vec![UiCommand::NotifyMessage(error)];
            }
        }
    } else if let Some(page) = state.find_downloader_mut(&page_id) {
        page.note(message.clone());
    }

    let mut commands = vec![
        UiCommand::GallerySeedLogDialog(gallery_seed_log_state(state, &page_id)),
        UiCommand::NotifyMessage(message),
    ];

    if let Some(page) = find_page(state, &page_id) {
        commands.push(UiCommand::UpdateDownloader(render::downloader_page_state(
            &state.controller,
            page,
        )));
    }

    commands
}

#[cfg(test)]
mod tests;
