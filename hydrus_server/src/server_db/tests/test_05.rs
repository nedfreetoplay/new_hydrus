use super::*;
use super::support::*;

#[test]
fn tag_parent_pending_and_petition_flows_are_counted_and_denied() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (service_id, _service_key) = create_tag_repository(&db, "tag petitions");
    let timestamp = hydrus_time::get_now();

    let child_id = db.get_master_tag_id("pending child").unwrap();
    let parent_id = db.get_master_tag_id("pending parent").unwrap();
    let reason_id = db.get_reason_id("test reason").unwrap();

    db.repository_pend_tag_parent(service_id, 1, child_id, parent_id, reason_id)
        .unwrap();

    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumPendingTagParents)
            .unwrap(),
        1
    );
    assert_eq!(
        db.get_service_info_specific(
            service_id,
            ServiceInfoType::NumActionableParentAddPetitions
        )
        .unwrap(),
        1
    );

    db.repository_deny_tag_parent_pend(service_id, child_id, parent_id)
        .unwrap();

    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumPendingTagParents)
            .unwrap(),
        0
    );

    db.repository_add_tag_parent(service_id, 1, child_id, parent_id, false, timestamp)
        .unwrap();
    db.get_service_info_specific(service_id, ServiceInfoType::NumTagParents)
        .unwrap();
    db.regenerate_service_info(Some(service_id), None).unwrap();

    let child_service_tag_id = db
        .repository_get_service_tag_id(service_id, child_id, timestamp)
        .unwrap();
    let parent_service_tag_id = db
        .repository_get_service_tag_id(service_id, parent_id, timestamp)
        .unwrap();

    db.repository_petition_tag_parent(
        service_id,
        1,
        child_service_tag_id,
        parent_service_tag_id,
        reason_id,
    )
    .unwrap();

    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumPetitionedTagParents)
            .unwrap(),
        1
    );

    db.repository_deny_tag_parent_petition(
        service_id,
        child_service_tag_id,
        parent_service_tag_id,
    )
    .unwrap();

    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumPetitionedTagParents)
            .unwrap(),
        0
    );
}

#[test]
fn file_petitions_are_counted_and_can_be_denied() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (service_id, _service_key) = create_file_repository(&db, "file petitions");
    let timestamp = hydrus_time::get_now();

    let source_path = dir.path.join("petition file.bin");
    std::fs::write(&source_path, b"petition").unwrap();
    let hash = vec![0xD1; 32];
    let file = ServerFileInput {
        hash: hash.clone(),
        size: 8,
        mime: Mime::OctetStream,
        width: None,
        height: None,
        duration_ms: None,
        num_frames: None,
        num_words: None,
        path: source_path,
        thumbnail: None,
    };
    db.repository_add_file(service_id, 1, &file, false, timestamp, None)
        .unwrap();

    let master_hash_id = db.get_master_hash_id(&hash).unwrap();
    let service_hash_id = db
        .repository_get_service_hash_id(service_id, master_hash_id, timestamp)
        .unwrap();
    let reason_id = db.get_reason_id("file petition reason").unwrap();

    db.repository_petition_files(service_id, 2, &[service_hash_id], reason_id)
        .unwrap();

    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumPetitionedFiles)
            .unwrap(),
        1
    );

    let summaries = db
        .repository_get_file_petitions_summary(service_id, 100, None, None)
        .unwrap();

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].content_type, ContentType::Files);

    db.repository_deny_file_petition(service_id, &[service_hash_id])
        .unwrap();

    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumPetitionedFiles)
            .unwrap(),
        0
    );
}
