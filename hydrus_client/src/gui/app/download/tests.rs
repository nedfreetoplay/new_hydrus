use std::path::Path;

use hydrus_core::hydrus_constants::Mime;
use serde_json::Value;

use super::*;
use crate::gui::app::model::{DownloaderPage, PageNode, SubscriptionEntry, SubscriptionsPage};
use crate::importing::ImportOptionsContainer;
use crate::importing::options::FileFilteringImportOptions;

fn test_state(directory: &Path) -> AppState {
    let _ = crate::client_options::register_serialisable_type();

    let controller = crate::controller::Controller::new(directory).unwrap();
    controller.init_model().unwrap();

    let mut state = AppState::new(std::sync::Arc::new(controller));
    state.root.children.clear();

    state
}

fn specific_png_options() -> ImportOptionsContainer {
    ImportOptionsContainer {
        file_filtering: Some(FileFilteringImportOptions {
            allowed_mimes: vec![Mime::Png],
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn gallery_and_simple_pages_use_post_urls_options() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_downloader_options_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let mut state = test_state(&directory);
    let mut page = DownloaderPage::new("p1", DownloaderKind::Gallery);
    page.import_options = specific_png_options();
    state.root.add(PageNode::Downloader(page));

    assert_eq!(downloader_caller(DownloaderKind::Gallery), "post_urls");
    assert_eq!(downloader_caller(DownloaderKind::Simple), "post_urls");
    assert_eq!(downloader_caller(DownloaderKind::Urls), "post_urls");
    assert_eq!(downloader_caller(DownloaderKind::Watcher), "watcher_urls");

    let effective = downloader_import_options(&state, "p1");
    let filter = effective.file_filtering_or_default();

    assert_eq!(filter.allowed_mimes, vec![Mime::Png]);
    assert!(
        import_options_summary(
            &state.find_downloader("p1").unwrap().import_options,
            "post_urls"
        )
        .contains("filetypes")
    );

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn saving_downloader_options_updates_the_page() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_downloader_options_save_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let mut state = test_state(&directory);
    state.root.add(PageNode::Downloader(DownloaderPage::new(
        "p1",
        DownloaderKind::Gallery,
    )));

    let payload = serde_json::json!({
        "target": "p1",
        "caller": "post_urls",
        "container": specific_png_options(),
    })
    .to_string();

    let commands = save_import_options(&mut state, &payload);
    let page = state.find_downloader("p1").unwrap();

    assert!(!page.import_options.is_empty());
    assert_eq!(
        page.import_options
            .file_filtering_or_default()
            .allowed_mimes,
        vec![Mime::Png]
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, UiCommand::UpdateDownloader(id, _) if id == "p1"))
    );

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn subscription_options_round_trip_and_render() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_subscription_options_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let mut state = test_state(&directory);
    state
        .root
        .add(PageNode::Subscriptions(SubscriptionsPage::new("s1")));
    let mut entry = SubscriptionEntry::new("sub", "https://example.com/search?q={query}");
    entry.query = "blue eyes".to_string();
    entry.import_options = specific_png_options();
    state.subscriptions.push(entry);

    let value: Value = serde_json::from_str(&render::subscriptions_json(&state, "")).unwrap();
    assert!(
        value["subscriptions"][0]["import_options_summary"]
            .as_str()
            .unwrap()
            .contains("filetypes")
    );

    let payload = serde_json::json!({
        "target": "sub",
        "container": specific_png_options(),
    })
    .to_string();
    let commands = save_subscription_import_options(&mut state, &payload);

    assert!(!state.subscriptions[0].import_options.is_empty());
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, UiCommand::UpdateSubscriptions(_, _)))
    );

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn subscription_editor_updates_the_selected_entry() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_subscription_editor_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let mut state = test_state(&directory);
    state
        .root
        .add(PageNode::Subscriptions(SubscriptionsPage::new("s1")));
    let mut entry = SubscriptionEntry::new("old", "https://example.com/search?q={query}");
    entry.query = "blue eyes".to_string();
    state.subscriptions.push(entry);

    let commands = edit_subscription_dialog(&state, "s1", "old");
    let dialog = match commands.first() {
        Some(UiCommand::SubscriptionEditor(json)) => serde_json::from_str::<Value>(json).unwrap(),
        other => panic!("expected the subscription editor, got {other:?}"),
    };

    assert_eq!(dialog["original"], "old");
    assert_eq!(dialog["query"], "blue eyes");
    assert!(dialog["sites"].as_array().unwrap().len() > 1);

    let payload = serde_json::json!({
        "original": "old",
        "name": "new",
        "url": "https://example.com/search?q={query}",
        "query": "red eyes",
        "site": "danbooru",
        "filter": ".png",
        "period": 7200,
        "paused": true,
    })
    .to_string();
    let commands = save_subscription_dialog(&mut state, "s1", &payload);

    assert!(state.subscriptions.iter().all(|entry| entry.name != "old"));
    let entry = state
        .subscriptions
        .iter()
        .find(|entry| entry.name == "new")
        .unwrap();
    assert_eq!(entry.query, "red eyes");
    assert_eq!(entry.site, "danbooru");
    assert_eq!(entry.filter, ".png");
    assert_eq!(entry.period, 7200);
    assert!(entry.paused);
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, UiCommand::UpdateSubscriptions(_, _)))
    );

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn subscription_editor_rejects_a_duplicate_name() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_subscription_editor_dupe_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let mut state = test_state(&directory);
    state.subscriptions.push(SubscriptionEntry::new(
        "one",
        "https://example.com/a?q={query}",
    ));
    state.subscriptions.push(SubscriptionEntry::new(
        "two",
        "https://example.com/b?q={query}",
    ));

    let payload = serde_json::json!({
        "original": "one",
        "name": "two",
        "url": "https://example.com/a?q={query}",
    })
    .to_string();
    let commands = save_subscription_dialog(&mut state, "", &payload);

    assert!(matches!(
        commands.first(),
        Some(UiCommand::NotifyMessage(message)) if message.contains("already exists")
    ));

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn saving_options_for_missing_targets_is_refused() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_downloader_options_missing_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let mut state = test_state(&directory);
    let payload = serde_json::json!({
        "target": "gone",
        "caller": "post_urls",
        "container": specific_png_options(),
    })
    .to_string();

    let commands = save_import_options(&mut state, &payload);

    assert!(matches!(
        commands.first(),
        Some(UiCommand::NotifyMessage(message)) if message.contains("gone")
    ));

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn simple_jobs_set_reorders_and_removes_pending_jobs() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_downloader_jobs_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let mut state = test_state(&directory);

    let mut page = DownloaderPage::new("p1", DownloaderKind::Simple);
    page.queue = vec![
        DownloadItem::new("https://example.com/a"),
        DownloadItem::new("https://example.com/b"),
        DownloadItem::new("https://example.com/c"),
    ];
    state.root.add(PageNode::Downloader(page));

    let reordered = r#"{"urls":["https://example.com/c","https://example.com/a"],"remove_missing":false}"#;
    let _ = jobs_set(&mut state, "p1", reordered);

    let page = state.find_downloader("p1").unwrap();
    let urls: Vec<&str> = page.queue.iter().map(|item| item.url.as_str()).collect();
    assert_eq!(
        urls,
        vec![
            "https://example.com/c",
            "https://example.com/a",
            "https://example.com/b"
        ]
    );

    let removed = r#"{"urls":["https://example.com/c"],"remove_missing":true}"#;
    let _ = jobs_set(&mut state, "p1", removed);

    let page = state.find_downloader("p1").unwrap();
    let urls: Vec<&str> = page.queue.iter().map(|item| item.url.as_str()).collect();
    assert_eq!(urls, vec!["https://example.com/c"]);

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn watcher_url_add_and_remove_updates_the_page() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_downloader_watcher_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let mut state = test_state(&directory);
    state.root.add(PageNode::Downloader(DownloaderPage::new(
        "p1",
        DownloaderKind::Watcher,
    )));

    let _ = set_watcher_url(&mut state, "p1", "https://example.com/gallery");

    let page = state.find_downloader("p1").unwrap();
    assert_eq!(page.url, "https://example.com/gallery");
    assert!(page.subject.is_empty());
    assert_eq!(page.last_check, 0);

    let _ = remove_watcher(&mut state, "p1");

    let page = state.find_downloader("p1").unwrap();
    assert!(page.url.is_empty());
    assert!(page.subject.is_empty());

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn set_site_updates_the_gallery_page() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_downloader_set_site_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let mut state = test_state(&directory);
    state.root.add(PageNode::Downloader(DownloaderPage::new(
        "p1",
        DownloaderKind::Gallery,
    )));

    let commands = set_site(&mut state, "p1", "safebooru");

    let page = state.find_downloader("p1").unwrap();
    assert_eq!(page.site, "safebooru");
    assert_eq!(page.file_limit, 2000);
    assert!(!commands.is_empty());

    drop(state);
    let _ = std::fs::remove_dir_all(directory);
}
