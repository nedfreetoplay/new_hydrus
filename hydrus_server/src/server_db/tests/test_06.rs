use super::*;
use super::support::*;

#[test]
fn mapping_petitions_are_counted_and_can_be_denied() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (service_id, _service_key) = create_tag_repository(&db, "mapping petitions");
    let timestamp = hydrus_time::get_now();

    let master_tag_id = db.get_master_tag_id("petition tag").unwrap();
    let master_hash_ids = db.get_master_hash_ids(&[vec![0xD2; 32]]).unwrap();

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
    let service_hash_id = db
        .repository_get_service_hash_ids(service_id, &master_hash_ids, timestamp)
        .unwrap()[0];
    let reason_id = db.get_reason_id("mapping petition reason").unwrap();

    db.repository_petition_mappings(
        service_id,
        2,
        service_tag_id,
        &[service_hash_id],
        reason_id,
    )
    .unwrap();

    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumPetitionedMappings)
            .unwrap(),
        1
    );

    let summaries = db
        .repository_get_mapping_petitions_summary(service_id, 100, None, None)
        .unwrap();

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].content_type, ContentType::Mappings);

    db.repository_deny_mapping_petition(service_id, service_tag_id, &[service_hash_id])
        .unwrap();

    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumPetitionedMappings)
            .unwrap(),
        0
    );
}

#[test]
fn client_to_server_mapping_update_and_petition_approval_work() {
    use hydrus_core::networking::hydrus_network::Content;

    let dir = TestDir::new();
    let db = make_db(&dir);
    let (service_id, service_key) = create_tag_repository(&db, "updates");
    let timestamp = hydrus_time::get_now();

    let admin_key = db
        .get_service_keys(Some(&[ServiceType::ServerAdmin]))
        .unwrap()[0]
        .clone();
    let access_key = db.get_access_key(&admin_key, b"init").unwrap();
    let account_key = db
        .get_account_key_from_access_key(&admin_key, &access_key)
        .unwrap();
    let account = db
        .get_account_from_account_key(&admin_key, &account_key)
        .unwrap();

    let content = Content::mapping("updated tag", vec![0xE1; 32]);
    let mut update = ClientToServerUpdate::new();
    update.add_content(ContentUpdate::Add, content.clone(), "accept mapping");

    db.repository_process_client_to_server_update(
        &service_key,
        &account,
        &update,
        timestamp,
    )
    .unwrap();

    let master_tag_id = db.get_master_tag_id("updated tag").unwrap();
    let service_tag_id = db
        .repository_get_service_tag_id(service_id, master_tag_id, timestamp)
        .unwrap();

    assert_eq!(
        db.repository_get_current_mappings_count(service_id, service_tag_id)
            .unwrap(),
        1
    );

    let master_hash_ids = db.get_master_hash_ids(&[vec![0xE2; 32]]).unwrap();
    let mut petition = Petition::new(
        Arc::new(account),
        PetitionHeader::new(
            ContentType::Mappings,
            ContentStatus::Pending,
            account_key,
            "approve mapping",
        ),
        vec![(
            ContentUpdate::Add,
            vec![Content::mapping("approved tag", vec![0xE2; 32])],
        )],
    );

    db.repository_approve_petition(&service_key, &mut petition, timestamp)
        .unwrap();

    let approved_tag_id = db.get_master_tag_id("approved tag").unwrap();
    let approved_service_tag_id = db
        .repository_get_service_tag_id(service_id, approved_tag_id, timestamp)
        .unwrap();

    assert_eq!(
        db.repository_get_current_mappings_count(service_id, approved_service_tag_id)
            .unwrap(),
        1
    );
    assert_eq!(
        db.repository_get_master_hash_ids(service_id, &master_hash_ids)
            .unwrap()
            .len(),
        1
    );
}
