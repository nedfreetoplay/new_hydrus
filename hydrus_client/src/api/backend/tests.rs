use super::*;
use hydrus_core::files::hydrus_file_handling::FileInfo;
use hydrus_core::hydrus_constants::Mime;

use crate::db::ClientDB;
use crate::db::{files_metadata, files_storage, mappings, master, master_tags, services};

fn setup() -> (ClientDB, ClientApiBackend, i64, i64, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_api_backend_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let db = ClientDB::new(&directory).unwrap();

    let file_service = services::get_or_add_service(
        db.db(),
        b"my files",
        ServiceType::LocalFileDomain,
        "my files",
    )
    .unwrap();
    files_storage::create_tables(db.db(), file_service).unwrap();

    let tag_service =
        services::get_or_add_service(db.db(), b"local tags", ServiceType::LocalTag, "local tags")
            .unwrap();
    mappings::create_tables(db.db(), tag_service).unwrap();

    let hash = vec![0xaa, 0xbb, 0xcc];
    let hash_id = master::get_hash_id(db.db(), &hash).unwrap();

    files_metadata::set_file_info(
        db.db(),
        hash_id,
        &FileInfo {
            size: 123,
            mime: Mime::Png,
            width: Some(10),
            height: Some(20),
            duration_ms: None,
            num_frames: None,
            has_audio: false,
            num_words: None,
        },
    )
    .unwrap();
    files_storage::add_files(
        db.db(),
        file_service,
        ContentStatus::Current,
        &[hash_id],
        None,
    )
    .unwrap();
    files_storage::set_inbox(db.db(), &[hash_id], true).unwrap();

    let services = services::get_services(db.db()).unwrap();
    let backend = ClientApiBackend::new(db.db().clone(), ClientFilesManager::default(), services);

    (db, backend, hash_id, tag_service, directory)
}

#[test]
fn search_and_metadata_read_the_client_database() {
    let (_db, backend, hash_id, _tag_service, directory) = setup();

    assert_eq!(
        backend.search_files(&["system:everything".to_string()]),
        vec![hash_id]
    );
    assert_eq!(
        backend.search_files(&["system:inbox".to_string()]),
        vec![hash_id]
    );
    assert!(
        backend
            .search_files(&["system:archive".to_string()])
            .is_empty()
    );

    let metadata = backend.file_metadata(&[hash_id]);
    assert_eq!(metadata[0]["file_id"], json!(hash_id));
    assert_eq!(metadata[0]["size"], json!(123));
    assert_eq!(metadata[0]["width"], json!(10));
    assert_eq!(metadata[0]["is_inbox"], json!(true));

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn add_tags_writes_current_mappings() {
    let (_db, backend, hash_id, tag_service, directory) = setup();

    let added = backend
        .add_tags(
            "local tags",
            &[hash_id],
            &["blue eyes".to_string(), "sky".to_string()],
        )
        .unwrap();

    assert_eq!(added, 2);
    let tags = backend.metadata(hash_id).unwrap();
    let values = tags["tags"]["local tags"].as_array().unwrap();
    assert!(values.contains(&json!("blue eyes")));
    assert!(values.contains(&json!("sky")));

    let tag_id = master_tags::get_tag_id(_db.db(), "blue eyes").unwrap();
    let sky_id = master_tags::get_tag_id(_db.db(), "sky").unwrap();
    let mut stored =
        mappings::get_tag_ids(_db.db(), tag_service, ContentStatus::Current, hash_id).unwrap();
    stored.sort_unstable();
    let mut expected = vec![tag_id, sky_id];
    expected.sort_unstable();

    assert_eq!(stored, expected);

    let _ = std::fs::remove_dir_all(directory);
}
