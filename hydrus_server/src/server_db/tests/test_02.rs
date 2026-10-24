use super::*;
use super::support::*;
use hydrus_core::hydrus_sessions::HydrusSessionManagerServer;

#[test]
fn ip_timestamp_round_trips_for_repository_files() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (_service_id, service_key, _master_hash_id) =
        add_file_to_new_repository(&db, &dir, vec![0xA2; 32], "ip_timestamp");
    let hash = vec![0xA2; 32];

    db.set_ip_timestamp(&service_key, &hash, "127.0.0.1", 123456)
        .unwrap();

    assert_eq!(
        db.get_ip_timestamp(&service_key, &hash).unwrap(),
        ("127.0.0.1".to_string(), 123456)
    );
}

#[test]
fn admin_registration_key_creates_a_usable_admin_account() {
    let dir = TestDir::new();
    let db = make_db(&dir);

    let admin_key = db
        .get_service_keys(Some(&[ServiceType::ServerAdmin]))
        .unwrap()[0]
        .clone();

    let account_types = db.get_account_types(&admin_key).unwrap();
    assert_eq!(account_types.len(), 2);
    assert!(
        account_types
            .iter()
            .any(|account_type| account_type.get_title() == "null account")
    );
    assert!(
        account_types
            .iter()
            .any(|account_type| account_type.get_title() == "administrator")
    );

    let access_key = db.get_access_key(&admin_key, b"init").unwrap();

    assert!(db.verify_access_key(&admin_key, &access_key).unwrap());

    let account_key = db
        .get_account_key_from_access_key(&admin_key, &access_key)
        .unwrap();

    assert!(db.verify_access_key(&admin_key, &access_key).unwrap());

    let account_id = db.get_account_id(&account_key).unwrap();

    assert!(account_id > 0);
    assert_eq!(
        db.get_account_key_from_account_id(account_id).unwrap(),
        account_key
    );

    let account = db
        .get_account_from_account_key(&admin_key, &account_key)
        .unwrap();

    assert_eq!(account.get_account_key(), account_key);
    assert_eq!(account.get_account_type().get_title(), "administrator");

    // The registration token is consumed when the account is created.
    assert!(db.get_access_key(&admin_key, b"init").is_err());
}

#[test]
fn unknown_registration_key_is_rejected() {
    let dir = TestDir::new();
    let db = make_db(&dir);

    let admin_key = db
        .get_service_keys(Some(&[ServiceType::ServerAdmin]))
        .unwrap()[0]
        .clone();

    let error = db.get_access_key(&admin_key, b"not-the-token").unwrap_err();

    assert!(error.is_a(ErrorKind::InsufficientCredentials));
}

#[test]
fn session_manager_can_add_and_restore_a_session() {
    let dir = TestDir::new();
    let db = make_db(&dir);

    let admin_key = db
        .get_service_keys(Some(&[ServiceType::ServerAdmin]))
        .unwrap()[0]
        .clone();
    let access_key = db.get_access_key(&admin_key, b"init").unwrap();
    let account_key = db
        .get_account_key_from_access_key(&admin_key, &access_key)
        .unwrap();

    let session_controller: Arc<dyn SessionController> =
        Arc::new(ServerSessionController::new(Arc::clone(&db)));
    let manager =
        HydrusSessionManagerServer::new(session_controller).expect("session manager");

    let (session_key, expires) = manager
        .add_session(&admin_key, &access_key)
        .expect("session is created");

    assert!(expires > hydrus_time::get_now());

    let account = manager
        .get_account(&admin_key, &session_key)
        .expect("session resolves to the account");

    assert_eq!(account.get_account_key(), account_key);

    manager
        .refresh_all_accounts(None)
        .expect("sessions reload from the database");

    let reloaded = manager
        .get_account(&admin_key, &session_key)
        .expect("reloaded session resolves");

    assert_eq!(reloaded.get_account_key(), account_key);

    let rows = db.get_sessions(Some(&admin_key)).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].session_key, session_key);
    assert_eq!(rows[0].account.get_account_key(), account_key);
}

#[test]
fn orphan_files_are_deferred_and_cleared() {
    let dir = TestDir::new();
    let db = make_db(&dir);

    let source_path = dir.path.join("orphan.bin");
    std::fs::write(&source_path, b"orphan").unwrap();

    let hash = vec![0xCD; 32];
    let file = ServerFileInput {
        hash: hash.clone(),
        size: 6,
        mime: Mime::OctetStream,
        width: None,
        height: None,
        duration_ms: None,
        num_frames: None,
        num_words: None,
        path: source_path,
        thumbnail: Some(b"thumb".to_vec()),
    };

    let master_hash_id = db.add_file(&file).unwrap();

    assert_eq!(
        db.defer_files_delete_if_now_orphan(&[master_hash_id], false, None)
            .unwrap(),
        1
    );
    assert_eq!(
        db.get_deferred_physical_delete().unwrap(),
        (Some(hash.clone()), Some(hash.clone()))
    );

    db.clear_deferred_physical_delete(Some(&hash), Some(&hash))
        .unwrap();

    assert_eq!(db.get_deferred_physical_delete().unwrap(), (None, None));
}
