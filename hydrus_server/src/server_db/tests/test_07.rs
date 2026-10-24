use super::*;
use super::support::*;

#[test]
fn account_info_and_mutations_work() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (service_id, service_key, access_key) =
        create_file_repository_with_access(&db, "account mutations");
    let timestamp = hydrus_time::get_now();

    let admin_service_key = db
        .get_service_keys(Some(&[ServiceType::ServerAdmin]))
        .unwrap()[0]
        .clone();
    let admin_access_key = db.get_access_key(&admin_service_key, b"init").unwrap();
    let _admin_account_key = db
        .get_account_key_from_access_key(&admin_service_key, &admin_access_key)
        .unwrap();
    let admin_account_key = db
        .get_account_key_from_access_key(&service_key, &access_key)
        .unwrap();
    let admin_account_id = db.get_account_id(&admin_account_key).unwrap();

    let source_path = dir.path.join("account file.bin");
    std::fs::write(&source_path, b"account file").unwrap();
    let hash = vec![0xF1; 32];
    let file = ServerFileInput {
        hash: hash.clone(),
        size: 12,
        mime: Mime::OctetStream,
        width: None,
        height: None,
        duration_ms: None,
        num_frames: None,
        num_words: None,
        path: source_path,
        thumbnail: None,
    };
    db.repository_add_file(
        service_id,
        admin_account_id,
        &file,
        false,
        timestamp,
        None,
    )
    .unwrap();

    let info = db.get_account_info(&service_key, admin_account_id).unwrap();
    assert_eq!(info.get("num_files"), Some(&1));

    let account_from_content = db
        .get_account_key_from_content(&service_key, &Content::files(vec![hash]))
        .unwrap();
    assert_eq!(account_from_content, admin_account_key);

    db.modify_account_set_message(&service_key, &admin_account_key, "hello")
        .unwrap();
    let account = db
        .get_account_from_account_key(&service_key, &admin_account_key)
        .unwrap();
    assert_eq!(account.get_message_and_timestamp().0, "hello");

    db.modify_account_ban(&service_key, &admin_account_key, "bad", None)
        .unwrap();
    assert!(
        db.get_account_from_account_key(&service_key, &admin_account_key)
            .unwrap()
            .is_banned()
    );

    db.modify_account_unban(&service_key, &admin_account_key)
        .unwrap();

    db.modify_account_expires(
        &service_key,
        &admin_account_key,
        Some(timestamp + 1000),
    )
    .unwrap();
    assert_eq!(
        db.get_account_from_account_key(&service_key, &admin_account_key)
            .unwrap()
            .get_expires(),
        Some(timestamp + 1000)
    );

    db.reward_accounts(service_id, i64::from(hydrus_core::hydrus_constants::SCORE_PETITION), &[(admin_account_id, 2)])
        .unwrap();
    assert_eq!(
        db.get_account_info(&service_key, admin_account_id)
            .unwrap()
            .get("petition_score"),
        Some(&2)
    );

    db.repository_nullify_history(&service_key, 0, timestamp + 1000)
        .unwrap();
    assert_eq!(
        db.get_account_info(&service_key, admin_account_id)
            .unwrap()
            .get("num_files"),
        Some(&0)
    );

    assert!(
        db.modify_account_delete_all_content(
            &service_key,
            &admin_account_key,
            &admin_account_key,
        )
        .unwrap()
    );
}

#[test]
fn account_types_can_be_replaced_and_accounts_reassigned() {
    use std::sync::Arc;
    use indexmap::IndexMap;
    use hydrus_core::networking::hydrus_networking::BandwidthRules;

    let dir = TestDir::new();
    let db = make_db(&dir);
    let (service_id, service_key, access_key) =
        create_file_repository_with_access(&db, "account types");
    let account_key = db
        .get_account_key_from_access_key(&service_key, &access_key)
        .unwrap();

    let current_types = db.get_account_types(&service_key).unwrap();
    let admin_type = current_types
        .iter()
        .find(|account_type| account_type.get_title() == "administrator")
        .unwrap()
        .as_ref()
        .clone();
    let new_type = AccountType::generate_new_account_type(
        "replacement type",
        IndexMap::new(),
        Arc::new(BandwidthRules::new()),
    );

    let mut replacements = std::collections::HashMap::new();
    replacements.insert(
        admin_type.get_account_type_key().to_vec(),
        new_type.get_account_type_key().to_vec(),
    );

    db.modify_account_types(&service_key, &[new_type.clone()], &replacements)
        .unwrap();

    let account = db
        .get_account_from_account_key(&service_key, &account_key)
        .unwrap();

    assert_eq!(account.get_account_type().get_title(), "replacement type");
    assert_eq!(db.get_account_types(&service_key).unwrap().len(), 2);
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumFiles)
            .unwrap(),
        0
    );
}

#[test]
fn service_period_options_are_persisted() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (_service_id, service_key) = create_tag_repository(&db, "service options");

    db.set_service_nullification_period(&service_key, 12345)
        .unwrap();
    db.set_service_update_period(&service_key, 67890)
        .unwrap();

    let service = db
        .get_services(None)
        .unwrap()
        .into_iter()
        .find(|service| service.get_service_key() == service_key)
        .unwrap();

    assert_eq!(service.get_nullification_period(), 12345);
    assert_eq!(service.get_update_period(), 67890);
}
