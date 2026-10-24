use super::*;
use super::support::*;

#[test]
fn current_repository_files_are_not_orphans() {
    let dir = TestDir::new();
    let db = make_db(&dir);

    let source_path = dir.path.join("current.bin");
    std::fs::write(&source_path, b"current").unwrap();

    let hash = vec![0xEF; 32];
    let file = ServerFileInput {
        hash: hash.clone(),
        size: 7,
        mime: Mime::OctetStream,
        width: None,
        height: None,
        duration_ms: None,
        num_frames: None,
        num_words: None,
        path: source_path,
        thumbnail: None,
    };

    let master_hash_id = db.add_file(&file).unwrap();
    let service = generate_service(
        vec![0x11; 32],
        ServiceType::FileRepository,
        "orphan test repository",
        DEFAULT_SERVICE_PORT,
        None,
    )
    .unwrap();
    let service_id = db.add_service(&service).unwrap();
    let (hash_map, _) = generate_repository_master_map_table_names(service_id);
    let (current_files, _, _, _, _) =
        generate_repository_files_table_names(service_id);

    let db_base = db.db().db_base().unwrap();
    db_base
        .execute(
            &format!(
                "INSERT INTO {hash_map} ( master_hash_id, hash_id_timestamp ) VALUES ( ?, ? );"
            ),
            &[&master_hash_id, &hydrus_time::get_now()],
        )
        .unwrap();
    let service_hash_id = db_base.get_last_row_id();
    db_base
        .execute(
            &format!(
                "INSERT INTO {current_files} ( service_hash_id, account_id, file_timestamp ) VALUES ( ?, ?, ? );"
            ),
            &[&service_hash_id, &1_i64, &hydrus_time::get_now()],
        )
        .unwrap();

    assert!(
        db.filter_orphan_master_hash_ids(&[master_hash_id], None)
            .unwrap()
            .is_empty()
    );
    assert!(!db.is_an_orphan(&hash).unwrap());
    assert_eq!(
        db.defer_files_delete_if_now_orphan(&[master_hash_id], false, None)
            .unwrap(),
        0
    );
}

#[test]
fn repository_add_and_delete_files_updates_tables_and_info() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (service_id, service_key) = create_file_repository(&db, "file ops");

    let source_path = dir.path.join("represented.bin");
    std::fs::write(&source_path, b"represented file").unwrap();

    let hash = vec![0xB1; 32];
    let file = ServerFileInput {
        hash: hash.clone(),
        size: 16,
        mime: Mime::OctetStream,
        width: None,
        height: None,
        duration_ms: None,
        num_frames: None,
        num_words: None,
        path: source_path,
        thumbnail: Some(b"thumbnail".to_vec()),
    };

    db.repository_add_file(
        service_id,
        1,
        &file,
        false,
        hydrus_time::get_now(),
        Some("127.0.0.1"),
    )
    .unwrap();

    assert_eq!(
        db.repository_has_file(&service_key, &hash).unwrap(),
        (true, Some(Mime::OctetStream))
    );
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumFiles)
            .unwrap(),
        1
    );
    assert_eq!(
        db.get_ip_timestamp(&service_key, &hash).unwrap().0,
        "127.0.0.1"
    );

    let (hash_map, _) = generate_repository_master_map_table_names(service_id);
    let master_hash_id = db.get_master_hash_id(&hash).unwrap();
    let service_hash_id = db
        .db()
        .db_base()
        .unwrap()
        .query_one(
            &format!(
                "SELECT service_hash_id FROM {hash_map} WHERE master_hash_id = ?;"
            ),
            &[&master_hash_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
        .unwrap();

    db.repository_delete_files(
        service_id,
        1,
        &[service_hash_id],
        hydrus_time::get_now(),
    )
    .unwrap();

    assert_eq!(
        db.repository_has_file(&service_key, &hash).unwrap(),
        (false, None)
    );
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumFiles)
            .unwrap(),
        0
    );
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumDeletedFiles)
            .unwrap(),
        1
    );
    assert_eq!(
        db.get_deferred_physical_delete().unwrap(),
        (Some(hash.clone()), Some(hash))
    );
}
