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
        hash_id: Some(7),
        blurhash: None,
        pixel_hash_hex: None,
        urls: vec![],
        local_file_services: vec!["my files".to_string()],
        is_trashed: false,
        can_post_ratings: true,
        is_local: true,
        has_static_image: true,
        image_width: None,
        image_height: None,
        mime_has_thumbnails: false,
        advanced_mode: false,
        global_mute: false,
        viewer_mute: false,
    }
}

fn labelled(entry: &crate::gui::qt::api::ffi::MenuEntry) -> String {
    if entry.is_separator {
        "<sep>".to_string()
    } else if entry.is_submenu {
        format!("<sub>{}", entry.label)
    } else {
        entry.label.clone()
    }
}

fn rows(menu: &crate::gui::qt::api::ffi::MenuBar) -> Vec<String> {
    menu.entries.iter().map(labelled).collect()
}

#[test]
fn empty_without_media() {
    let menu = viewer_menu(&ViewerMenuFacts::default());
    assert!(menu.entries.is_empty());
}

#[test]
fn full_top_level_order_inbox_file() {
    let menu = viewer_menu(&facts());
    let names = rows(&menu);
    assert_eq!(
        names,
        vec![
            "<sub>jpeg, 1.0 MB",
            "jpeg, 1.0 MB",
            "<sep>",
            "<sub>volume",
            "mute global",
            "<sep>",
            "mute media viewer",
            "<sep>",
            "volume",
            "<sep>",
            "archive",
            "<sep>",
            "delete from my files",
            "<sep>",
            "<sub>manage",
            "tags",
            "ratings",
            "notes (2)",
            "times",
            "force filetype",
            "<sub>viewing stats",
            "clear",
            "<sub>urls",
            "manage",
            "<sub>open",
            "in a new page",
            "in a new duplicate filter page",
            "<sub>similar files in a new page",
            "<sep>",
            "exact match",
            "very similar",
            "similar",
            "speculative",
            "<sep>",
            "custom",
            "<sep>",
            "in web browser",
            "in another program",
            "properties",
            "<sub>share",
            "export files",
            "<sep>",
            "copy file",
            "copy path",
            "<sub>copy hash",
            "sha256 (ab)",
            "md5",
            "sha1",
            "sha512",
            "copy file id (7)",
            "copy bitmap",
            "<sep>",
            "<sub>player",
            "This is a Unknown Media Player - let hydev know please.",
        ]
    );
}

#[test]
fn manage_and_niche_rows_are_disabled_without_mechanisms() {
    let menu = viewer_menu(&facts());

    let disabled = |label: &str| {
        menu.entries
            .iter()
            .any(|entry| !entry.is_submenu && !entry.is_separator && entry.label == label
                && !entry.enabled)
    };
    let enabled = |label: &str, action: UiAction| {
        menu.entries
            .iter()
            .any(|entry| entry.label == label && entry.enabled && entry.action == action)
    };

    for label in [
        "mute global",
        "mute media viewer",
        "volume",
        "ratings",
        "force filetype",
        "clear",
        "in a new page",
        "in a new duplicate filter page",
        "exact match",
        "in another program",
        "export files",
        "copy file",
        "copy bitmap",
        "md5",
        "sha1",
        "sha512",
        "This is a Unknown Media Player - let hydev know please.",
    ] {
        assert!(disabled(label), "{label} must be disabled");
    }

    for (label, action) in [
        ("archive", UiAction::ArchiveFiles),
        ("delete from my files", UiAction::DeleteFilesFromService),
        ("tags", UiAction::ManageTags),
        ("times", UiAction::EditTimestamps),
        ("copy path", UiAction::CopyFilePath),
        ("in web browser", UiAction::OpenFileInWebBrowser),
        ("properties", UiAction::OpenFileInNativeProperties),
    ] {
        assert!(enabled(label, action), "{label} must be enabled");
    }
}

#[test]
fn archived_file_shows_inbox_not_archive() {
    let mut owned = facts();
    owned.inbox = false;
    owned.archived = true;
    owned.note_count = 0;
    let menu = viewer_menu(&owned);
    let names = rows(&menu);

    assert!(names.contains(&"inbox".to_string()));
    assert!(!names.contains(&"archive".to_string()));
    assert!(names.contains(&"notes".to_string()));
}

#[test]
fn delete_submenu_with_several_local_services() {
    let mut owned = facts();
    owned.local_file_services = vec!["a files".to_string(), "b files".to_string()];
    let menu = viewer_menu(&owned);
    let names = rows(&menu);

    let index = names.iter().position(|name| name == "<sub>delete").expect("delete submenu");
    assert_eq!(names[index + 1], "from a files");
    assert_eq!(names[index + 2], "from b files");

    for entry in viewer_menu(&owned).entries {
        if entry.label == "from a files" {
            assert_eq!(entry.target, "a files");
            assert_eq!(entry.payload, "ab");
            assert_eq!(entry.action, UiAction::DeleteFilesFromService);
        }
    }
}

#[test]
fn trashed_file_rows() {
    let mut owned = facts();
    owned.is_trashed = true;
    owned.local_file_services = vec![];
    let menu = viewer_menu(&owned);

    let physically = menu
        .entries
        .iter()
        .find(|entry| entry.label == "delete physically now")
        .expect("delete physically now");
    assert!(!physically.enabled);

    let undelete = menu
        .entries
        .iter()
        .find(|entry| entry.label == "undelete")
        .expect("undelete");
    assert!(undelete.enabled);
    assert_eq!(undelete.action, UiAction::UndeleteFiles);
    assert_eq!(undelete.payload, "ab");
}

#[test]
fn urls_submenu_rows() {
    let mut owned = facts();
    owned.urls = vec!["https://a".to_string(), "https://b".to_string()];
    let menu = viewer_menu(&owned);

    let visit = menu
        .entries
        .iter()
        .find(|entry| entry.label == "https://a" && entry.action == UiAction::OpenKnownUrl)
        .expect("visit url item");
    assert!(visit.enabled);
    assert_eq!(visit.payload, "https://a");

    let copy = menu
        .entries
        .iter()
        .find(|entry| entry.label == "this file's 2 urls")
        .expect("aggregate copy");
    assert!(copy.enabled);
    assert_eq!(copy.action, UiAction::CopyKnownUrl);
    assert_eq!(copy.payload, "https://a\nhttps://b");

    assert!(menu
        .entries
        .iter()
        .any(|entry| entry.label == "force metadata refetch" && entry.is_submenu));
}

#[test]
fn hash_actions_carry_algo_in_target() {
    let owned = facts();
    let menu = viewer_menu(&owned);

    let sha256 = menu
        .entries
        .iter()
        .find(|entry| entry.label == "sha256 (ab)")
        .expect("sha256 row");
    assert_eq!(sha256.action, UiAction::CopyFileHash);
    assert_eq!(sha256.target, "sha256");
    assert_eq!(sha256.payload, "ab");

    let file_id = menu
        .entries
        .iter()
        .find(|entry| entry.label == "copy file id (7)")
        .expect("copy file id");
    assert_eq!(file_id.action, UiAction::CopyFileId);
    assert_eq!(file_id.payload, "ab");
}

#[test]
fn page_actions_carry_page_and_hash() {
    let menu = viewer_menu(&facts());
    for entry in &menu.entries {
        if entry.is_separator || entry.is_submenu {
            continue;
        }
        match entry.action {
            UiAction::ArchiveFiles | UiAction::InboxFiles | UiAction::ManageTags => {
                assert_eq!(entry.target, "p1");
                assert_eq!(entry.payload, "ab");
            }
            UiAction::NotesManage => {
                assert_eq!(entry.payload, "ab");
            }
            _ => {}
        }
    }
}

#[test]
fn file_browser_item_needs_advanced_mode() {
    let mut owned = facts();
    owned.advanced_mode = false;
    let menu = viewer_menu(&owned);
    let names = rows(&menu);
    assert!(!names.contains(&"in file browser".to_string()));

    owned.advanced_mode = true;
    let menu = viewer_menu(&owned);
    let labels = rows(&menu);
    assert!(labels.contains(&"in file browser".to_string()));
}

#[test]
fn non_static_images_have_no_similar_submenu() {
    let mut owned = facts();
    owned.has_static_image = false;
    let menu = viewer_menu(&owned);
    let names = rows(&menu);
    assert!(!names
        .iter()
        .any(|name| name == "<sub>similar files in a new page"));
}

#[test]
fn muted_labels_flip_from_options() {
    let mut owned = facts();
    owned.global_mute = true;
    owned.viewer_mute = true;
    let menu = viewer_menu(&owned);
    let names = rows(&menu);
    assert!(names.contains(&"unmute global".to_string()));
    assert!(names.contains(&"unmute media viewer".to_string()));
}

#[test]
fn notes_label_counts() {
    assert_eq!(notes_label(0), "notes");
    assert_eq!(notes_label(3), "notes (3)");
}

#[test]
fn source_lookup_bitmap_only_when_large() {
    use super::source_lookup_resolution;
    // Small images: Python adds no row.
    let mut small = facts();
    small.image_width = Some(800);
    small.image_height = Some(600);
    assert_eq!(source_lookup_resolution(&small), None);
    assert!(
        !rows(&viewer_menu(&small))
            .iter()
            .any(|name| name.starts_with("copy source lookup bitmap"))
    );

    // Large images: fitted into 1024x1024 preserving aspect.
    let mut large = facts();
    large.image_width = Some(2048);
    large.image_height = Some(1024);
    assert_eq!(source_lookup_resolution(&large), Some((1024, 512)));
    assert!(
        rows(&viewer_menu(&large))
            .iter()
            .any(|name| name == "copy source lookup bitmap (1024x512)")
    );

    // Non-local or non-image files: no bitmap rows at all.
    let mut remote = facts();
    remote.is_local = false;
    assert!(
        !rows(&viewer_menu(&remote))
            .iter()
            .any(|name| name.contains("bitmap"))
    );
}

#[test]
fn thumbnail_bitmap_row_needs_thumbnail_mime() {
    let mut owned = facts();
    owned.has_static_image = false;
    owned.mime_has_thumbnails = true;
    assert!(
        rows(&viewer_menu(&owned))
            .iter()
            .any(|name| name == "copy thumbnail bitmap")
    );

    let plain = facts();
    assert!(
        !rows(&viewer_menu(&plain))
            .iter()
            .any(|name| name == "copy thumbnail bitmap")
    );
}
