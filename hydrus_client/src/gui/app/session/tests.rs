use super::*;
use crate::gui::app::model::{Notebook, PageNode, PlaceholderPage, QueryPage};

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

    let session = SessionFile {
        version: 1,
        root: from_notebook(&root),
        active: Some("p1".to_string()),
    };

    let text = serde_json::to_string(&session).unwrap();
    let parsed: SessionFile = serde_json::from_str(&text).unwrap();
    let restored = to_notebook(&parsed.root);

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
fn session_file_loads_from_disk() {
    let path = std::env::temp_dir().join(format!(
        "hydrus_client_app_session_{}_{}.json",
        std::process::id(),
        rand::random::<u64>()
    ));

    let session = SessionFile {
        version: 1,
        root: from_notebook(&crate::gui::app::model::default_root()),
        active: Some("p1".to_string()),
    };

    std::fs::write(&path, serde_json::to_string(&session).unwrap()).unwrap();

    let notebook = load(&path).expect("session loads");

    assert_eq!(notebook.children.len(), 1);
    assert_eq!(notebook.current.as_deref(), Some("p1"));

    let _ = std::fs::remove_file(path);
}
