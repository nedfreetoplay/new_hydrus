use super::{ViewerMenuFacts, notes_label, viewer_menu};
use crate::gui::qt::api::ffi::UiAction;

fn facts() -> ViewerMenuFacts {
    ViewerMenuFacts {
        has_media: true,
        inbox: true,
        archived: false,
        note_count: 2,
        info_title: "jpeg, 1.0 MB".to_string(),
        page_id: "p1".to_string(),
        hash_hex: "ab".to_string(),
    }
}

#[test]
fn empty_without_media() {
    let menu = viewer_menu(&ViewerMenuFacts::default());
    assert!(menu.entries.is_empty());
}

#[test]
fn order_and_labels_inbox_file() {
    let menu = viewer_menu(&facts());
    let names: Vec<String> = menu
        .entries
        .iter()
        .map(|entry| {
            if entry.is_separator {
                "<sep>".to_string()
            } else if entry.is_submenu {
                format!("<sub>{}", entry.label)
            } else {
                entry.label.clone()
            }
        })
        .collect();
    assert_eq!(
        names,
        vec![
            "<sub>jpeg, 1.0 MB",
            "jpeg, 1.0 MB",
            "<sep>",
            "archive",
            "<sep>",
            "delete",
            "<sep>",
            "<sub>manage",
            "tags",
            "notes (2)",
            "times",
            "viewing stats",
        ]
    );
    assert!(menu.entries.iter().all(|entry| entry.depth <= 1));
}

#[test]
fn archived_file_shows_inbox_not_archive() {
    let mut owned = facts();
    owned.inbox = false;
    owned.archived = true;
    owned.note_count = 0;
    let menu = viewer_menu(&owned);
    let names: Vec<String> = menu
        .entries
        .iter()
        .filter(|entry| !entry.is_separator && !entry.is_submenu)
        .map(|entry| entry.label.clone())
        .collect();
    assert!(names.contains(&"inbox".to_string()));
    assert!(!names.contains(&"archive".to_string()));
    assert!(names.contains(&"notes".to_string()));
}

#[test]
fn actions_carry_page_and_hash() {
    let menu = viewer_menu(&facts());
    for entry in &menu.entries {
        if entry.is_separator || entry.is_submenu || entry.action == UiAction::None {
            continue;
        }
        if entry.action == UiAction::EditTimestamps {
            continue;
        }
        assert_eq!(entry.target, "p1");
        assert_eq!(entry.payload, "ab");
    }
}

#[test]
fn notes_label_counts() {
    assert_eq!(notes_label(0), "notes");
    assert_eq!(notes_label(3), "notes (3)");
}
