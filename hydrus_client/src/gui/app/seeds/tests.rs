use std::path::Path;

use super::*;
use crate::gui::app::model::DownloadItem;
use crate::gui::app::model::{DownloaderKind, PageNode};
use serde_json::Value;

fn test_state(directory: &Path, kind: DownloaderKind) -> (AppState, DownloaderPage) {
    let _ = crate::client_options::register_serialisable_type();

    let controller = crate::controller::Controller::new(directory).unwrap();
    controller.init_model().unwrap();

    let mut state = AppState::new(std::sync::Arc::new(controller));
    state.root.children.clear();
    let page = DownloaderPage::new("p1", kind);
    state.root.add(PageNode::Downloader(page.clone()));

    (state, page)
}

fn with_queue(state: &mut AppState) {
    let page = state.find_downloader_mut("p1").unwrap();

    let mut done = DownloadItem::new("https://example.com/a.jpg");
    done.mark(DownloadItemStatus::Done, "imported");

    let mut failed = DownloadItem::new("https://example.com/b.jpg");
    failed.mark(DownloadItemStatus::Error, "404");

    let pending = DownloadItem::new("https://example.com/c.jpg");

    page.queue = vec![done, failed, pending];
}

#[test]
fn file_seed_cache_json_counts_statuses() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_seed_cache_json_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let (mut state, _) = test_state(&directory, DownloaderKind::Urls);
    with_queue(&mut state);

    let value: Value = serde_json::from_str(&file_seed_cache_json(&state, "p1")).unwrap();

    assert_eq!(value["kind"], "file");
    assert_eq!(value["seeds"].as_array().unwrap().len(), 3);
    assert_eq!(value["counts"]["done"], 1);
    assert_eq!(value["counts"]["failed"], 1);
    assert_eq!(value["counts"]["pending"], 1);
    assert!(value["subtitle"].as_str().unwrap().contains("3 file seed"));

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn file_seed_cache_remove_and_clear_finished() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_seed_cache_actions_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let (mut state, _) = test_state(&directory, DownloaderKind::Urls);
    with_queue(&mut state);

    let payload = serde_json::json!({
        "page": "p1",
        "action": "remove",
        "urls": ["https://example.com/b.jpg"],
    })
    .to_string();
    let commands = file_seed_action(&mut state, &payload);

    assert!(matches!(
        commands.first(),
        Some(UiCommand::FileSeedCacheDialog(_))
    ));
    assert_eq!(state.find_downloader("p1").unwrap().queue.len(), 2);
    assert!(
        state
            .find_downloader("p1")
            .unwrap()
            .queue
            .iter()
            .all(|item| item.url != "https://example.com/b.jpg")
    );

    let commands = file_seed_action(
        &mut state,
        &serde_json::json!({"page": "p1", "action": "clear_finished"}).to_string(),
    );
    let updated: Value = match commands.first() {
        Some(UiCommand::FileSeedCacheDialog(json)) => serde_json::from_str(json).unwrap(),
        other => panic!("expected the updated dialog, got {other:?}"),
    };

    assert_eq!(updated["seeds"].as_array().unwrap().len(), 1);
    assert_eq!(updated["seeds"][0]["status"], "pending");

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn gallery_seed_log_deduplicates_by_url() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_gallery_seed_log_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let (mut state, _) = test_state(&directory, DownloaderKind::Gallery);

    {
        let page = state.find_downloader_mut("p1").unwrap();
        page.upsert_gallery_seed("https://example.com/gallery", "working", "fetching");
        page.upsert_gallery_seed("https://example.com/gallery", "success", "20 file(s) found");
        page.upsert_gallery_seed("https://example.com/post/1", "failed", "could not fetch");
    }

    let page = state.find_downloader_mut("p1").unwrap();
    assert_eq!(page.gallery_seeds.len(), 2);
    assert_eq!(
        page.gallery_seeds
            .iter()
            .find(|seed| seed.url.ends_with("gallery"))
            .unwrap()
            .status,
        "success"
    );

    let value: Value = serde_json::from_str(&gallery_seed_log_json(&state, "p1")).unwrap();

    assert_eq!(value["kind"], "gallery");
    assert_eq!(value["counts"]["done"], 1);
    assert_eq!(value["counts"]["failed"], 1);
    assert_eq!(value["seeds"][0]["url"], "https://example.com/post/1");

    let commands = gallery_seed_action(
        &mut state,
        &serde_json::json!({"page": "p1", "action": "clear_finished"}).to_string(),
    );
    assert!(matches!(
        commands.first(),
        Some(UiCommand::GallerySeedLogDialog(_))
    ));
    assert!(
        state
            .find_downloader("p1")
            .unwrap()
            .gallery_seeds
            .is_empty()
    );

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn gallery_seed_log_only_opens_for_gallery_pages() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_gallery_seed_log_kind_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let (state, _) = test_state(&directory, DownloaderKind::Urls);

    let commands = open_gallery_seed_log(&state, "p1");

    assert!(matches!(
        commands.first(),
        Some(UiCommand::NotifyMessage(message)) if message.contains("no gallery seed log")
    ));

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn gallery_seed_log_round_trips_through_the_page_model() {
    let mut page = DownloaderPage::new("p1", DownloaderKind::Watcher);
    page.upsert_gallery_seed("https://example.com/watch", "success", "5 file(s) found");

    let json = serde_json::to_value(&page).unwrap();
    let loaded: DownloaderPage = serde_json::from_value(json).unwrap();

    assert_eq!(loaded.gallery_seeds.len(), 1);
    assert_eq!(loaded.gallery_seeds[0].status, "success");
    assert!(loaded.queue.is_empty());
}
