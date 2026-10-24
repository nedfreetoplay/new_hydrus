use super::*;
use super::support::*;

#[test]
fn repository_updates_are_created_from_current_content() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (service_id, service_key, access_key) =
        create_file_repository_with_access(&db, "update generation");

    let account_key = db
        .get_account_key_from_access_key(&service_key, &access_key)
        .unwrap();
    let account_id = db.get_account_id(&account_key).unwrap();
    let timestamp = hydrus_time::get_now();

    let source_path = dir.path.join("update source.bin");
    std::fs::write(&source_path, b"update source").unwrap();

    let file = ServerFileInput {
        hash: vec![0xC1; 32],
        size: 13,
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
        account_id,
        &file,
        false,
        timestamp,
        None,
    )
    .unwrap();

    let updates = db
        .repository_generate_updates(service_id, timestamp - 1, timestamp + 1)
        .unwrap();

    assert!(!updates.is_empty());

    let update_hashes = db
        .repository_create_update(&service_key, timestamp - 1, timestamp + 1)
        .unwrap();

    assert!(!update_hashes.is_empty());

    let files = ServerFiles::new(db.get_files_dir());

    for update_hash in &update_hashes {
        assert!(files.get_file_path(update_hash).unwrap().exists());
    }
}

#[test]
fn repository_create_update_is_empty_without_new_rows() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (_service_id, service_key) = create_file_repository(&db, "empty updates");

    let update_hashes = db
        .repository_create_update(&service_key, 0, 0)
        .unwrap();

    assert!(update_hashes.is_empty());
}

#[test]
fn schema_migrations_recreate_missing_repository_tables() {
    let dir = TestDir::new();
    let db = make_db(&dir);
    let (service_id, _service_key) = create_file_repository(&db, "migrations");

    let update_table = generate_repository_update_table_name(service_id);
    let (_, _, _, _, ip_addresses) =
        generate_repository_files_table_names(service_id);

    let db_base = db.db().db_base().unwrap();
    db_base
        .execute(&format!("DROP TABLE {update_table};"), &[])
        .unwrap();
    db_base
        .execute(&format!("DROP TABLE {ip_addresses};"), &[])
        .unwrap();

    assert!(db.ensure_schema_migrations().unwrap());
    assert!(db.table_exists(&update_table).unwrap());
    assert!(db.table_exists(&ip_addresses).unwrap());
}
