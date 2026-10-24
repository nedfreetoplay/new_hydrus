use super::*;
use super::support::*;

#[test]
fn repository_mappings_add_delete_and_count() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (service_id, _service_key) = create_tag_repository(&db, "mappings");

    let master_tag_id = db.get_master_tag_id("blue sky").unwrap();
    let hashes = vec![vec![0xC1; 32], vec![0xC2; 32]];
    let master_hash_ids = db.get_master_hash_ids(&hashes).unwrap();
    let timestamp = hydrus_time::get_now();

    db.repository_add_mappings(
        service_id,
        1,
        master_tag_id,
        &master_hash_ids,
        false,
        timestamp,
    )
    .unwrap();

    let service_tag_id = db
        .repository_get_service_tag_id(service_id, master_tag_id, timestamp)
        .unwrap();

    assert_eq!(
        db.repository_get_master_tag_id(service_id, service_tag_id)
            .unwrap(),
        master_tag_id
    );
    assert!(
        db.repository_service_tag_id_exists(service_id, master_tag_id)
            .unwrap()
    );
    assert_eq!(
        db.repository_get_current_mappings_count(service_id, service_tag_id)
            .unwrap(),
        2
    );
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumMappings)
            .unwrap(),
        2
    );
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumTags)
            .unwrap(),
        1
    );

    let service_hash_ids = db
        .repository_get_service_hash_ids(service_id, &master_hash_ids, timestamp)
        .unwrap();

    db.repository_delete_mappings(
        service_id,
        1,
        service_tag_id,
        &[service_hash_ids[0]],
        timestamp,
    )
    .unwrap();

    assert_eq!(
        db.repository_get_current_mappings_count(service_id, service_tag_id)
            .unwrap(),
        1
    );
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumMappings)
            .unwrap(),
        1
    );
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumDeletedMappings)
            .unwrap(),
        1
    );
}

#[test]
fn repository_tag_parents_and_siblings_add_delete_and_count() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (service_id, _service_key) = create_tag_repository(&db, "tag relations");
    let timestamp = hydrus_time::get_now();

    let child_master_tag_id = db.get_master_tag_id("child").unwrap();
    let parent_master_tag_id = db.get_master_tag_id("parent").unwrap();
    let bad_master_tag_id = db.get_master_tag_id("bad").unwrap();
    let good_master_tag_id = db.get_master_tag_id("good").unwrap();

    db.repository_add_tag_parent(
        service_id,
        1,
        child_master_tag_id,
        parent_master_tag_id,
        false,
        timestamp,
    )
    .unwrap();
    db.repository_add_tag_sibling(
        service_id,
        1,
        bad_master_tag_id,
        good_master_tag_id,
        false,
        timestamp,
    )
    .unwrap();

    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumTagParents)
            .unwrap(),
        1
    );
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumTagSiblings)
            .unwrap(),
        1
    );

    let child_service_tag_id = db
        .repository_get_service_tag_id(service_id, child_master_tag_id, timestamp)
        .unwrap();
    let parent_service_tag_id = db
        .repository_get_service_tag_id(service_id, parent_master_tag_id, timestamp)
        .unwrap();
    let bad_service_tag_id = db
        .repository_get_service_tag_id(service_id, bad_master_tag_id, timestamp)
        .unwrap();
    let good_service_tag_id = db
        .repository_get_service_tag_id(service_id, good_master_tag_id, timestamp)
        .unwrap();

    db.repository_delete_tag_parent(
        service_id,
        1,
        child_service_tag_id,
        parent_service_tag_id,
        timestamp,
    )
    .unwrap();
    db.repository_delete_tag_sibling(
        service_id,
        1,
        bad_service_tag_id,
        good_service_tag_id,
        timestamp,
    )
    .unwrap();

    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumTagParents)
            .unwrap(),
        0
    );
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumTagSiblings)
            .unwrap(),
        0
    );
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumDeletedTagParents)
            .unwrap(),
        1
    );
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumDeletedTagSiblings)
            .unwrap(),
        1
    );
}
