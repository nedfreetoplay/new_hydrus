use super::*;
use super::support::*;

#[test]
fn table_name_helpers_match_python() {
    assert_eq!(
        generate_repository_master_map_table_names(7),
        (
            "external_master.repository_hash_id_map_7".to_string(),
            "external_master.repository_tag_id_map_7".to_string()
        )
    );
    assert_eq!(
        generate_repository_files_table_names(7).0,
        "current_files_7"
    );
    assert_eq!(
        generate_repository_mappings_table_names(7).0,
        "external_mappings.current_mappings_7"
    );
    assert_eq!(
        generate_repository_tag_parents_table_names(7).0,
        "current_tag_parents_7"
    );
    assert_eq!(
        generate_repository_tag_siblings_table_names(7).0,
        "current_tag_siblings_7"
    );
    assert_eq!(generate_repository_update_table_name(7), "updates_7");
}

#[test]
fn fresh_server_db_creates_the_schema_and_admin_service() {
    let dir = TestDir::new();
    let db = make_db(&dir);

    assert!(db.table_exists("services").unwrap());
    assert!(db.table_exists("files_info").unwrap());
    assert!(db.table_exists("accounts").unwrap());

    let services = db.get_services(None).unwrap();

    assert_eq!(services.len(), 1);
    assert_eq!(services[0].get_name(), "server admin");
    assert_eq!(services[0].get_service_type(), ServiceType::ServerAdmin);
    assert_eq!(services[0].get_port(), DEFAULT_SERVER_ADMIN_PORT);

    assert!(db.get_files_dir().join("00").is_dir());
    assert!(db.get_files_dir().join("ff").is_dir());
}

#[test]
fn service_queries_round_trip_and_filter() {
    let dir = TestDir::new();
    let db = make_db(&dir);

    let admin_key = db.get_service_keys(Some(&[ServiceType::ServerAdmin])).unwrap()[0].clone();
    let admin_id = db.get_service_id(&admin_key).unwrap();

    assert_eq!(db.get_service_name(admin_id).unwrap(), "server admin");
    assert_eq!(
        db.get_service_type(admin_id).unwrap(),
        ServiceType::ServerAdmin
    );
    assert_eq!(db.get_service_key(admin_id).unwrap(), admin_key);

    assert_eq!(
        db.get_service_ids(Some(&[ServiceType::ServerAdmin]))
            .unwrap(),
        vec![admin_id]
    );
    assert!(
        db.get_service_ids(Some(&[ServiceType::FileRepository]))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn file_addition_stores_bytes_thumbnail_and_metadata() {
    let dir = TestDir::new();
    let db = make_db(&dir);

    let source_path = dir.path.join("source.bin");
    std::fs::write(&source_path, b"data").unwrap();

    let hash = vec![0xAB; 32];
    let file = ServerFileInput {
        hash: hash.clone(),
        size: 4,
        mime: Mime::OctetStream,
        width: Some(10),
        height: Some(20),
        duration_ms: None,
        num_frames: None,
        num_words: None,
        path: source_path,
        thumbnail: Some(b"thumbnail".to_vec()),
    };

    let master_hash_id = db.add_file(&file).unwrap();

    assert_eq!(db.get_master_hash_id(&hash).unwrap(), master_hash_id);
    assert_eq!(db.get_hash(master_hash_id).unwrap(), hash);
    assert!(db.hash_exists(&hash).unwrap());
    assert!(db.master_hash_exists(&hash).unwrap());

    let server_files = ServerFiles::new(db.get_files_dir());
    let stored_path = server_files.get_expected_file_path(&hash);
    let thumbnail_path = server_files.get_expected_thumbnail_path(&hash);

    assert_eq!(std::fs::read(stored_path).unwrap(), b"data");
    assert_eq!(std::fs::read(thumbnail_path).unwrap(), b"thumbnail");

    let db_base = db.db().db_base().unwrap();
    let (size, mime, width, height) = db_base
        .query_one(
            "SELECT size, mime, width, height FROM files_info WHERE master_hash_id = ?;",
            &[&master_hash_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .unwrap()
        .unwrap();

    assert_eq!(size, 4);
    assert_eq!(mime, i64::from(Mime::OctetStream.as_int()));
    assert_eq!((width, height), (10, 20));
}

#[test]
fn master_tags_are_created_once_and_read_back() {
    let dir = TestDir::new();
    let db = make_db(&dir);

    let tag_id = db.get_master_tag_id("blue sky").unwrap();

    assert_eq!(db.get_master_tag_id("blue sky").unwrap(), tag_id);
    assert_eq!(db.get_tag(tag_id).unwrap(), "blue sky");
    assert!(db.master_tag_exists("blue sky").unwrap());
    assert!(!db.master_tag_exists("green sky").unwrap());
}

#[test]
fn service_info_regeneration_counts_repository_rows() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (_service_id, service_key, _master_hash_id) =
        add_file_to_new_repository(&db, &dir, vec![0xA1; 32], "service_info");

    let service_id = db.get_service_id(&service_key).unwrap();

    db.regenerate_service_info(Some(service_id), None).unwrap();

    let info = db.get_service_info(&service_key).unwrap();

    assert_eq!(
        info.get(&i64::from(ServiceInfoType::NumFileHashes.as_int())),
        Some(&1)
    );
    assert_eq!(
        info.get(&i64::from(ServiceInfoType::NumFiles.as_int())),
        Some(&1)
    );
    assert_eq!(
        db.get_service_info_specific(service_id, ServiceInfoType::NumFiles)
            .unwrap(),
        1
    );

    db.update_service_info(service_id, ServiceInfoType::NumFiles, 2)
        .unwrap();

    let info = db.get_service_info(&service_key).unwrap();

    assert_eq!(
        info.get(&i64::from(ServiceInfoType::NumFiles.as_int())),
        Some(&3)
    );
}
