use super::*;
use crate::controller::Controller;
use crate::db::{mappings, master, master_tags};
use hydrus_core::hydrus_constants::ContentStatus;

#[test]
fn rules_round_trip_through_json() {
    let rules = vec![
        AutoResolutionRule {
            name: "keep big".to_string(),
            comparator: "larger_file".to_string(),
            action: "keep_better".to_string(),
        },
        AutoResolutionRule {
            name: "no more".to_string(),
            comparator: "always".to_string(),
            action: "not_duplicates".to_string(),
        },
    ];

    let text = serde_json::to_string(&rules).unwrap();
    let parsed: Vec<AutoResolutionRule> = serde_json::from_str(&text).unwrap();

    assert_eq!(parsed, rules);
}

#[test]
fn tag_comparator_reads_the_database() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_auto_resolution_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();

    let db = controller.db().db();
    let a = master::get_hash_id(db, &[1]).unwrap();
    let b = master::get_hash_id(db, &[2]).unwrap();
    let service = controller
        .services()
        .get_service(b"local tags")
        .unwrap()
        .service_id;
    let tag = master_tags::get_tag_id(db, "blue eyes").unwrap();

    mappings::add_mappings(db, service, ContentStatus::Current, a, &[tag]).unwrap();

    let a_hash = master::get_hash(db, a).unwrap().unwrap();
    let b_hash = master::get_hash(db, b).unwrap().unwrap();

    assert_eq!(
        compare(&controller, "more_tags", &a_hash, &b_hash),
        Some(std::cmp::Ordering::Greater)
    );
    assert_eq!(
        compare(&controller, "fewer_tags", &a_hash, &b_hash),
        Some(std::cmp::Ordering::Less)
    );

    drop(controller);
    let _ = std::fs::remove_dir_all(directory);
}
