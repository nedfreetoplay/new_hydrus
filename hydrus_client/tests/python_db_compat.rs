//! Compatibility smoke test against a Python-shaped `client.db`.
//!
//! The fixture mirrors the table layout the Python client writes for the
//! subset the Rust port already understands. It is deliberately created with
//! raw sqlite, not with `ClientDB`, so it exercises the loader path.

use hydrus_client::db::{ClientDB, files_metadata, master, services};
use hydrus_core::hydrus_constants::{Mime, ServiceType};
use rusqlite::{Connection, params};

fn create_python_shaped_db(path: &std::path::Path) -> (Vec<u8>, Vec<u8>) {
    let connection = Connection::open(path).unwrap();

    connection
        .execute_batch(
            "
            CREATE TABLE version ( version INTEGER );
            CREATE TABLE options ( options TEXT );
            CREATE TABLE services (
                service_id INTEGER PRIMARY KEY,
                service_key BLOB_BYTES,
                service_type INTEGER,
                name TEXT,
                dictionary_string TEXT
            );
            CREATE TABLE service_info (
                service_id INTEGER,
                info_type INTEGER,
                info BLOB_BYTES,
                PRIMARY KEY ( service_id, info_type )
            );
            CREATE TABLE hashes (
                hash_id INTEGER PRIMARY KEY,
                hash BLOB_BYTES UNIQUE
            );
            CREATE TABLE files_info (
                hash_id INTEGER PRIMARY KEY,
                size INTEGER,
                mime INTEGER,
                width INTEGER,
                height INTEGER,
                duration INTEGER,
                num_frames INTEGER,
                has_audio INTEGER,
                num_words INTEGER
            );
            CREATE TABLE json_dumps (
                name TEXT PRIMARY KEY,
                dump_type INTEGER,
                version INTEGER,
                dump TEXT
            );
            ",
        )
        .unwrap();

    let service_key = b"my files".to_vec();
    let hash = vec![0xde, 0xad, 0xbe, 0xef];

    connection
        .execute("INSERT INTO version ( version ) VALUES ( 620 )", [])
        .unwrap();
    connection
        .execute(
            "INSERT INTO options ( options ) VALUES ( 'legacy options' )",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO services ( service_id, service_key, service_type, name, dictionary_string )
             VALUES ( 1, ?1, ?2, 'my files', '' )",
            params![service_key, ServiceType::LocalFileDomain.as_int()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO hashes ( hash_id, hash ) VALUES ( 3, ?1 )",
            params![hash],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO files_info
             ( hash_id, size, mime, width, height, duration, num_frames, has_audio, num_words )
             VALUES ( 3, 12345, ?1, 640, 480, NULL, NULL, 0, NULL )",
            params![Mime::Png.as_int()],
        )
        .unwrap();

    (service_key, hash)
}

#[test]
fn opens_and_reads_python_shaped_database() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_python_db_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("client.db");

    let (service_key, hash) = create_python_shaped_db(&path);

    let db = ClientDB::from_path(&path).unwrap();

    assert_eq!(db.get_version().unwrap(), 620);
    assert_eq!(
        db.read_options_text().unwrap().as_deref(),
        Some("legacy options")
    );

    let service = services::get_service(db.db(), &service_key)
        .unwrap()
        .unwrap();
    assert_eq!(service.name, "my files");
    assert_eq!(service.service_type, ServiceType::LocalFileDomain);

    let hash_id = master::get_hash_id(db.db(), &hash).unwrap();
    let file_info = files_metadata::get_file_info(db.db(), hash_id)
        .unwrap()
        .unwrap();
    assert_eq!(file_info.size, 12345);
    assert_eq!(file_info.mime, Mime::Png);
    assert_eq!(file_info.width, Some(640));

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn update_db_advances_old_version() {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_python_db_update_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("client.db");

    create_python_shaped_db(&path);

    let db = ClientDB::from_path(&path).unwrap();
    let version = db.update_db().unwrap();

    assert!(version >= hydrus_core::hydrus_constants::SOFTWARE_VERSION as i64);

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
