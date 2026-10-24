use super::*;
use crate::db::{ClientDB, master_tags};
use hydrus_core::hydrus_constants::ServiceType;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_content_updates_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    let db = ClientDB::new(&directory).unwrap();
    services::get_or_add_service(db.db(), b"local tags", ServiceType::LocalTag, "local tags")
        .unwrap();
    services::get_or_add_service(
        db.db(),
        b"my files",
        ServiceType::LocalFileDomain,
        "my files",
    )
    .unwrap();

    (db, directory)
}

#[test]
fn applies_mapping_updates() {
    let (db, directory) = database();
    let hash = vec![0xaa];
    let package = ContentUpdatePackage::from_updates(vec![
        ContentUpdate::new(
            ContentAction::Add,
            b"local tags".to_vec(),
            ContentType::Mappings,
        )
        .with_hashes([hash.clone()])
        .with_tags(["blue eyes"]),
    ]);

    let result = apply_content_update_package(db.db(), &package).unwrap();
    let hash_id = master::get_hash_id(db.db(), &hash).unwrap();
    let service = services::get_service(db.db(), b"local tags")
        .unwrap()
        .unwrap();
    let tag_id = master_tags::get_tag_id(db.db(), "blue eyes").unwrap();

    assert_eq!(result.applied, 1);
    assert_eq!(
        mappings::get_tag_ids(db.db(), service.service_id, ContentStatus::Current, hash_id)
            .unwrap(),
        vec![tag_id]
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn applies_file_and_url_updates() {
    let (db, directory) = database();
    let hash = vec![0xbb];
    let package = ContentUpdatePackage::from_updates(vec![
        ContentUpdate::new(ContentAction::Add, b"my files".to_vec(), ContentType::Files)
            .with_hashes([hash.clone()]),
        ContentUpdate::new(ContentAction::Add, b"my files".to_vec(), ContentType::Urls)
            .with_hashes([hash.clone()])
            .with_urls(["https://example.com/a"]),
    ]);

    apply_content_update_package(db.db(), &package).unwrap();
    let hash_id = master::get_hash_id(db.db(), &hash).unwrap();
    let service = services::get_service(db.db(), b"my files")
        .unwrap()
        .unwrap();

    assert_eq!(
        files_storage::get_hash_ids(db.db(), service.service_id, ContentStatus::Current).unwrap(),
        vec![hash_id]
    );
    assert_eq!(url_map::get_urls(db.db(), hash_id).unwrap().len(), 1);

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn missing_services_are_skipped() {
    let (db, directory) = database();
    let package = ContentUpdatePackage::from_updates(vec![
        ContentUpdate::new(
            ContentAction::Add,
            b"missing service".to_vec(),
            ContentType::Files,
        )
        .with_hashes([vec![1]]),
    ]);

    let result = apply_content_update_package(db.db(), &package).unwrap();

    assert_eq!(result.skipped, 1);

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
