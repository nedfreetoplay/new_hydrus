use super::*;
use crate::gui::app::model::{
    DownloadItem, DownloadItemStatus, DownloaderKind, DownloaderPage, Notebook, PageNode,
    PlaceholderPage, QueryPage, SubscriptionEntry, SubscriptionsPage,
};
use crate::gui::session::UnifiedSession;

fn round_trip(notebook: &Notebook) -> Notebook {
    let value = serde_json::to_value(from_notebook(notebook)).unwrap();
    let node: SessionNode = serde_json::from_value(value).unwrap();

    to_notebook(&node)
}

#[test]
fn notebook_round_trips_through_the_session_format() {
    let mut root = Notebook::new("root", "top page notebook");

    let mut query = QueryPage::new("p1", "files", "my files");
    query.query = "blue eyes".to_string();
    root.add(PageNode::Query(query));

    let mut nested = Notebook::new("n2", "page of pages");
    nested.add(PageNode::Placeholder(PlaceholderPage {
        id: "p3".to_string(),
        title: "duplicates processing".to_string(),
        message: "pending".to_string(),
        status: String::new(),
    }));
    root.add(PageNode::Notebook(nested));

    let restored = round_trip(&root);

    assert_eq!(restored.children.len(), 2);

    match &restored.children[0] {
        PageNode::Query(page) => {
            assert_eq!(page.id, "p1");
            assert_eq!(page.query, "blue eyes");
            assert_eq!(page.location, "my files");
        }
        _ => panic!("first node was not a query"),
    }

    match &restored.children[1] {
        PageNode::Notebook(notebook) => {
            assert_eq!(notebook.id, "n2");
            assert_eq!(notebook.children.len(), 1);
        }
        _ => panic!("second node was not a notebook"),
    }
}

#[test]
fn downloaders_and_subscriptions_round_trip() {
    let mut root = Notebook::new("root", "top page notebook");

    let mut downloader = DownloaderPage::new("p2", DownloaderKind::Gallery);
    downloader.url = "https://example.com/search?q={query}".to_string();
    downloader.query = "blue eyes".to_string();
    downloader.site = "danbooru".to_string();
    downloader.filter = ".jpg".to_string();
    downloader.auto_watch = true;
    downloader.check_period = 900;
    downloader.queue.push(DownloadItem {
        url: "https://example.com/a.jpg".to_string(),
        status: DownloadItemStatus::Done,
        note: "ok".to_string(),
    });
    downloader.results.push(vec![0xab, 0xcd]);
    root.add(PageNode::Downloader(downloader));

    root.add(PageNode::Subscriptions(SubscriptionsPage::new("p3")));

    let restored = round_trip(&root);

    match &restored.children[0] {
        PageNode::Downloader(page) => {
            assert_eq!(page.kind, DownloaderKind::Gallery);
            assert_eq!(page.query, "blue eyes");
            assert_eq!(page.site, "danbooru");
            assert_eq!(page.filter, ".jpg");
            assert!(page.auto_watch);
            assert_eq!(page.check_period, 900);
            assert_eq!(page.queue[0].status, DownloadItemStatus::Done);
            assert_eq!(page.results, vec![vec![0xab, 0xcd]]);
        }
        _ => panic!("downloader did not round-trip"),
    }

    let subscription = SubscriptionEntry::new("daily", "https://example.com/search?q={query}");
    let value = serde_json::to_value(&subscription).unwrap();
    let restored: SubscriptionEntry = serde_json::from_value(value).unwrap();

    assert_eq!(restored.name, "daily");
}

#[test]
fn import_folder_pages_round_trip() {
    let mut root = Notebook::new("root", "top page notebook");
    root.add(PageNode::ImportFolders(
        crate::gui::app::model::ImportFoldersPage::new("p4"),
    ));

    let restored = round_trip(&root);

    assert!(matches!(restored.children[0], PageNode::ImportFolders(_)));
}

#[test]
fn session_file_loads_from_disk() {
    let path = std::env::temp_dir().join(format!(
        "hydrus_client_app_session_{}_{}.json",
        std::process::id(),
        rand::random::<u64>()
    ));

    let root = crate::gui::app::model::default_root();
    let unified = UnifiedSession {
        version: UnifiedSession::VERSION,
        legacy: None,
        app: Some(serde_json::to_value(from_notebook(&root)).unwrap()),
        active: Some("p1".to_string()),
        subscriptions: Vec::new(),
        session_name: None,
    };

    unified.save(&path).unwrap();

    let notebook = load(&path).expect("session loads");

    assert_eq!(notebook.children.len(), 1);
    assert_eq!(notebook.current.as_deref(), Some("p1"));

    let _ = std::fs::remove_file(path);
}
