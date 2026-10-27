use super::*;

#[test]
fn sanitises_names_that_cannot_be_files() {
    assert_eq!(sanitise_name("my: session?"), "my_ session_");
    assert_eq!(sanitise_name("  ..  "), "session");
    assert_eq!(sanitise_name("normal name"), "normal name");
}

#[test]
fn lists_saved_sessions_with_page_counts() {
    let folder = std::env::temp_dir().join(format!(
        "hydrus_client_sessions_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    std::fs::create_dir_all(&folder).unwrap();

    let session = serde_json::json!({
        "version": 3,
        "app": {
            "root": {
                "id": "root",
                "kind": "notebook",
                "title": "top page notebook",
                "children": [
                    {"id": "p1", "kind": "query", "title": "files"},
                    {"id": "n2", "kind": "notebook", "title": "group", "children": [
                        {"id": "p3", "kind": "query", "title": "second"}
                    ]}
                ]
            }
        }
    });

    std::fs::write(
        folder.join("one.json"),
        serde_json::to_string_pretty(&session).unwrap(),
    )
    .unwrap();

    let sessions = list_in(&folder);

    assert_eq!(
        sessions,
        vec![SessionInfo {
            name: "one".to_string(),
            pages: 2
        }]
    );

    let _ = std::fs::remove_dir_all(folder);
}
