use super::*;
use crate::controller::Controller;
use hydrus_core::hydrus_constants::{Mime, ServiceType};

fn test_controller() -> (Controller, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_gui_search_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();

    (controller, directory)
}

fn tag_service_id(controller: &Controller) -> i64 {
    controller
        .services()
        .get_services(Some(&[ServiceType::LocalTag]))
        .first()
        .map(|service| service.service_id)
        .unwrap()
}

#[test]
fn parses_tags_namespaces_wildcards_and_negation() {
    let (controller, directory) = test_controller();
    let db = controller.db().db();
    master_tags::get_tag_id(db, "blue eyes").unwrap();

    let plan = super::plan(db, "blue eyes -sky species:* foo* system:width > 80").unwrap();

    assert_eq!(plan.tags.len(), 4);
    assert_eq!(plan.tags[0].kind, TagTermKind::Tag);
    assert_eq!(plan.tags[0].text, "blue eyes");
    assert!(plan.tags[0].inclusive);
    assert_eq!(plan.tags[1].kind, TagTermKind::Tag);
    assert!(!plan.tags[1].inclusive);
    assert_eq!(plan.tags[2].kind, TagTermKind::Namespace);
    assert_eq!(plan.tags[2].text, "species");
    assert_eq!(plan.tags[3].kind, TagTermKind::Wildcard);
    assert_eq!(plan.predicates.len(), 1);
    assert_eq!(
        plan.predicates[0].predicate_type,
        PredicateType::SystemWidth
    );

    assert!(super::plan(db, "system:not a real predicate").is_err());

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn numeric_system_predicates_test_file_info() {
    let (controller, directory) = test_controller();
    let db = controller.db().db();
    let tag_service = tag_service_id(&controller);
    let info = FileInfo {
        size: 2048,
        mime: Mime::Png,
        width: Some(100),
        height: Some(50),
        duration_ms: None,
        num_frames: None,
        has_audio: false,
        num_words: None,
    };

    let plan = super::plan(
        db,
        "system:width > 80 system:height < 100 system:ratio = 2:1 system:filesize < 4KB system:num pixels > 1000 pixels",
    )
    .unwrap();

    assert_eq!(plan.system.other_system.len(), 5);

    for predicate in &plan.system.other_system {
        assert_eq!(
            matches_system(&controller, db, 1, &info, tag_service, None, predicate).unwrap(),
            Some(true),
            "{predicate:?} did not match"
        );
    }

    let plan = super::plan(db, "system:width > 200").unwrap();

    assert_eq!(
        matches_system(
            &controller,
            db,
            1,
            &info,
            tag_service,
            None,
            &plan.system.other_system[0]
        )
        .unwrap(),
        Some(false)
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn unsupported_system_predicates_are_ignored() {
    let (controller, directory) = test_controller();
    let db = controller.db().db();
    let tag_service = tag_service_id(&controller);
    let info = FileInfo {
        size: 1,
        mime: Mime::Png,
        width: Some(1),
        height: Some(1),
        duration_ms: None,
        num_frames: None,
        has_audio: false,
        num_words: None,
    };

    let plan = super::plan(db, "system:has exif").unwrap();

    assert_eq!(
        matches_system(
            &controller,
            db,
            1,
            &info,
            tag_service,
            None,
            &plan.system.other_system[0]
        )
        .unwrap(),
        None
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn mime_and_hash_filters_use_file_info() {
    let (controller, directory) = test_controller();
    let db = controller.db().db();
    let info = FileInfo {
        size: 1,
        mime: Mime::Png,
        width: Some(1),
        height: Some(1),
        duration_ms: None,
        num_frames: None,
        has_audio: false,
        num_words: None,
    };
    let hash = vec![0x01; 32];

    let plan = super::plan(
        db,
        "system:filetype = png system:hash = 0101010101010101010101010101010101010101010101010101010101010101",
    )
    .unwrap();

    assert!(matches_mime_or_hash(&hash, &info, &plan.system).unwrap());

    assert!(!matches_mime_or_hash(&[9, 9], &info, &plan.system).unwrap());

    let _ = std::fs::remove_dir_all(directory);
}
