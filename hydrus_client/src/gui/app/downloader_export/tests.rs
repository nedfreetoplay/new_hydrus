use super::*;

#[test]
fn downloader_definitions_round_trip() {
    let mut page = DownloaderPage::new("p1", DownloaderKind::Gallery);
    page.title = "example gallery".to_string();
    page.url = "https://example.com/gallery/{query}".to_string();
    page.query = "wolf".to_string();
    page.site = "generic".to_string();
    page.filter = "full".to_string();
    page.subject = "wolves".to_string();
    page.check_period = 7200;
    page.auto_watch = true;
    page.queue.push(crate::gui::app::model::DownloadItem {
        url: "https://example.com/1".to_string(),
        status: crate::gui::app::model::DownloadItemStatus::Pending,
        note: String::new(),
        created: 0,
        modified: 0,
    });

    let json = to_json(&page);
    let restored = from_json(&json, "p9").unwrap();

    assert_eq!(restored.id, "p9");
    assert_eq!(restored.title, "example gallery");
    assert_eq!(restored.kind, DownloaderKind::Gallery);
    assert_eq!(restored.url, "https://example.com/gallery/{query}");
    assert_eq!(restored.query, "wolf");
    assert_eq!(restored.site, "generic");
    assert_eq!(restored.filter, "full");
    assert_eq!(restored.subject, "wolves");
    assert_eq!(restored.check_period, 7200);
    assert!(restored.auto_watch);
    assert!(restored.queue.is_empty());
}

#[test]
fn invalid_downloader_definitions_are_rejected() {
    assert!(
        from_json(&serde_json::json!({"kind": "gallery"}), "p1")
            .unwrap_err()
            .contains("Hydrus downloader")
    );

    assert!(
        from_json(
            &serde_json::json!({"format": FORMAT, "kind": "mystery"}),
            "p1"
        )
        .unwrap_err()
        .contains("unknown kind")
    );
}
