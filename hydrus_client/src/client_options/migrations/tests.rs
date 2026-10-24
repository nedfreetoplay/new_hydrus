use super::*;

#[test]
fn v1_media_view_is_upgraded_through_all_versions() {
    let info = json!({
        "media_view": {
            "0": [0, 1, true, false, 3, 4]
        }
    });

    let (_, migrated) = migrate(1, info).unwrap();
    let entry = migrated["media_view"]["0"].as_array().unwrap();

    assert_eq!(entry.len(), 7);
    assert_eq!(entry[0], json!(0));
    assert_eq!(entry[3], json!(1));
}

#[test]
fn v2_portable_paths_use_forward_slashes() {
    let info = json!({
        "client_files_locations_ideal_weights": [["C:\\hydrus\\files", 1]],
        "client_files_locations_full_size_thumbnail_override": "C:\\hydrus\\thumbs"
    });

    let (_, migrated) = migrate(2, info).unwrap();

    assert_eq!(
        migrated["client_files_locations_ideal_weights"][0][0],
        json!("C:/hydrus/files")
    );
    assert_eq!(
        migrated["client_files_locations_full_size_thumbnail_override"],
        json!("C:/hydrus/thumbs")
    );
}

#[test]
fn old_versions_are_upgraded_to_current() {
    let (version, _) = migrate(5, json!({})).unwrap();

    assert_eq!(version, super::super::SERIALISABLE_VERSION);
}

#[test]
fn v3_media_view_tuples_become_v4_seven_tuples() {
    let info = json!({
        "media_view": {
            "0": [0, 1, [1.0, 0, 0]]
        }
    });

    let (_, migrated) = migrate(3, info).unwrap();
    let entry = migrated["media_view"]["0"].as_array().unwrap();

    assert_eq!(entry.len(), 7);
    assert_eq!(entry[1], json!(false));
    assert_eq!(entry[2], json!(false));
}

#[test]
fn v6_qmediaplayer_action_is_replaced() {
    let info = json!({
        "media_view": {
            "0": [8, false, false, 8, false, false, []]
        }
    });

    let (_, migrated) = migrate(6, info).unwrap();

    assert_eq!(migrated["media_view"]["0"][0], json!(9));
    assert_eq!(migrated["media_view"]["0"][3], json!(9));
}

#[test]
fn v7_browser_path_moves_and_rewrites_placeholder() {
    let info = json!({
        "noneable_strings": { "web_browser_path": "firefox %path%" },
        "media_launch": { "0": "vlc" }
    });

    let (_, migrated) = migrate(7, info).unwrap();

    assert_eq!(
        migrated["web_browser_launch_paths"][0],
        json!("firefox %url%")
    );
    assert!(migrated.get("media_launch").is_none());
    assert!(migrated.get("open_externally_launch_paths").is_some());
}
