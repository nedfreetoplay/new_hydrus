use super::*;
use crate::controller::Controller;
use crate::db::{files_storage, mappings, master, master_tags};
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
fn autocomplete_entries_come_from_the_tag_index() {
    let (controller, directory) = test_controller();
    let db = controller.db().db();
    let tag_service = tag_service_id(&controller);
    let file_service = controller
        .services()
        .get_service(b"my files")
        .unwrap()
        .service_id;

    let first = master::get_hash_id(db, &[0x41; 32]).unwrap();
    let second = master::get_hash_id(db, &[0x42; 32]).unwrap();

    for hash_id in [first, second] {
        files_storage::add_files(
            db,
            file_service,
            ContentStatus::Current,
            &[hash_id],
            Some(hydrus_core::hydrus_time::get_now()),
        )
        .unwrap();
    }

    let blue = master_tags::get_tag_id(db, "blue eyes").unwrap();
    let sky = master_tags::get_tag_id(db, "blue sky").unwrap();
    let creator = master_tags::get_tag_id(db, "creator:someone").unwrap();

    mappings::add_mappings(
        db,
        tag_service,
        ContentStatus::Current,
        first,
        &[blue, creator],
    )
    .unwrap();
    mappings::add_mappings(
        db,
        tag_service,
        ContentStatus::Current,
        second,
        &[blue, sky],
    )
    .unwrap();

    let entries = autocomplete_entries(db, tag_service, "blu", 10);
    let tags: Vec<(&str, Option<i64>, &str)> = entries
        .iter()
        .map(|entry| (entry.text.as_str(), entry.count, entry.category))
        .collect();

    assert!(tags.contains(&("blue eyes", Some(2), "tags")));
    assert!(tags.contains(&("blue sky", Some(1), "tags")));

    let namespaces = autocomplete_entries(db, tag_service, "crea", 10);
    let namespace = namespaces
        .iter()
        .find(|entry| entry.category == "namespaces")
        .unwrap();

    assert_eq!(namespace.text, "creator:");
    assert_eq!(namespace.count, Some(1));

    let system = autocomplete_entries(db, tag_service, "system:in", 10);

    assert!(system.iter().any(|entry| entry.text == "system:inbox"));
    assert!(system.iter().all(|entry| entry.category == "system"));

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn url_and_viewing_stats_predicates_are_parsed_and_evaluated() {
    use crate::db::{files_viewing_stats, url_map};

    let (controller, directory) = test_controller();
    let db = controller.db().db();
    let tag_service = tag_service_id(&controller);
    let file_service = controller
        .services()
        .get_service(b"my files")
        .unwrap()
        .service_id;
    let hash = master::get_hash_id(db, &[0x61; 32]).unwrap();

    files_storage::add_files(
        db,
        file_service,
        ContentStatus::Current,
        &[hash],
        Some(hydrus_core::hydrus_time::get_now()),
    )
    .unwrap();
    url_map::add_url(db, hash, "https://example.com/gallery/123").unwrap();
    files_viewing_stats::add_viewing_stats(db, hash, 0, 3, 1500, Some(1000)).unwrap();

    let info = FileInfo {
        size: 1,
        mime: Mime::Png,
        width: Some(10),
        height: Some(10),
        duration_ms: None,
        num_frames: None,
        has_audio: false,
        num_words: None,
    };

    let plan = super::plan(
        db,
        &controller.services(),
        "system:has url matching regex example\\.com system:has url with domain example.com system:media views > 2",
    )
    .unwrap();

    assert_eq!(plan.system.other_system.len(), 3);

    for predicate in &plan.system.other_system {
        assert_eq!(
            matches_system(
                &controller,
                db,
                hash,
                &info,
                tag_service,
                Some(file_service),
                predicate,
            )
            .unwrap(),
            Some(true),
            "{predicate:?} did not match"
        );
    }

    let plan = super::plan(
        db,
        &controller.services(),
        "system:does not have url with domain other.example",
    )
    .unwrap();

    assert_eq!(
        matches_system(
            &controller,
            db,
            hash,
            &info,
            tag_service,
            Some(file_service),
            &plan.system.other_system[0],
        )
        .unwrap(),
        Some(true)
    );

    let plan = super::plan(
        db,
        &controller.services(),
        "system:media viewtime > 1 second",
    )
    .unwrap();

    assert_eq!(
        matches_system(
            &controller,
            db,
            hash,
            &info,
            tag_service,
            Some(file_service),
            &plan.system.other_system[0],
        )
        .unwrap(),
        Some(true)
    );

    let plan = super::plan(
        db,
        &controller.services(),
        "system:file service is currently in my files",
    )
    .unwrap();

    assert_eq!(plan.system.other_system.len(), 1);
    assert_eq!(
        matches_system(
            &controller,
            db,
            hash,
            &info,
            tag_service,
            Some(file_service),
            &plan.system.other_system[0],
        )
        .unwrap(),
        Some(true)
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn parses_tags_namespaces_wildcards_and_negation() {
    let (controller, directory) = test_controller();
    let db = controller.db().db();
    master_tags::get_tag_id(db, "blue eyes").unwrap();

    let plan = super::plan(
        db,
        &controller.services(),
        "blue eyes -sky species:* foo* system:width > 80",
    )
    .unwrap();

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

    assert!(super::plan(db, &controller.services(), "system:not a real predicate").is_err());

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
        &controller.services(),
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

    let plan = super::plan(db, &controller.services(), "system:width > 200").unwrap();

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

    let plan = super::plan(db, &controller.services(), "system:has exif").unwrap();

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
        &controller.services(),
        "system:filetype = png system:hash = 0101010101010101010101010101010101010101010101010101010101010101",
    )
    .unwrap();

    assert!(matches_mime_or_hash(&hash, &info, &plan.system).unwrap());

    assert!(!matches_mime_or_hash(&[9, 9], &info, &plan.system).unwrap());

    let _ = std::fs::remove_dir_all(directory);
}
