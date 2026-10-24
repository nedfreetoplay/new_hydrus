use super::*;
use crate::client_options::ClientOptions;
use hydrus_core::files::hydrus_file_handling::FileInfo;
use hydrus_core::hydrus_constants::{Mime, ServiceType};

fn temporary_db() -> (ClientDB, PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_db_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    let db = ClientDB::new(&directory).unwrap();

    (db, directory)
}

#[test]
fn creates_the_initial_schema_and_version() {
    let (db, directory) = temporary_db();

    assert!(db.db().table_exists("version").unwrap());
    assert!(db.db().table_exists("services").unwrap());
    assert_eq!(
        db.get_version().unwrap(),
        hydrus_core::hydrus_constants::SOFTWARE_VERSION as i64
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hashes_are_idempotent() {
    let (db, directory) = temporary_db();
    let hash = vec![0xaa, 0xbb];

    let first = master::get_hash_id(db.db(), &hash).unwrap();
    let second = master::get_hash_id(db.db(), &hash).unwrap();

    assert_eq!(first, second);
    assert_eq!(master::get_hash(db.db(), first).unwrap(), Some(hash));

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn services_round_trip() {
    let (db, directory) = temporary_db();

    let service_id = services::get_or_add_service(
        db.db(),
        b"my files",
        ServiceType::LocalFileDomain,
        "my files",
    )
    .unwrap();
    let service = services::get_service(db.db(), b"my files")
        .unwrap()
        .unwrap();

    assert_eq!(service.service_id, service_id);
    assert_eq!(service.name, "my files");

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn file_info_round_trips() {
    let (db, directory) = temporary_db();
    let hash_id = master::get_hash_id(db.db(), &[1, 2, 3]).unwrap();
    let info = FileInfo {
        size: 1024,
        mime: Mime::Png,
        width: Some(100),
        height: Some(200),
        duration_ms: None,
        num_frames: None,
        has_audio: false,
        num_words: None,
    };

    files_metadata::set_file_info(db.db(), hash_id, &info).unwrap();

    assert_eq!(
        files_metadata::get_file_info(db.db(), hash_id).unwrap(),
        Some(info)
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn client_options_round_trip() {
    let _ = crate::client_options::register_serialisable_type();
    let (db, directory) = temporary_db();
    let mut options = ClientOptions::new();
    options.set_boolean("advanced_mode", true);

    db.set_serialisable("client options", &options).unwrap();
    let loaded = db.get_serialisable("client options").unwrap().unwrap();
    let loaded = loaded.as_any().downcast_ref::<ClientOptions>().unwrap();

    assert!(loaded.get_boolean("advanced_mode"));

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
