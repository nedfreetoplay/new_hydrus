
use super::*;
use crate::db::ClientDB;
use crate::db::master::get_hash_id;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_servicepaths_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn filenames_round_trip_per_service() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[1]).unwrap();

    set_service_filename(db.db(), 1, hash_id, "one.png").unwrap();
    set_service_filename(db.db(), 2, hash_id, "two.png").unwrap();

    assert_eq!(
        get_service_filename(db.db(), 1, hash_id)
            .unwrap()
            .as_deref(),
        Some("one.png")
    );
    assert_eq!(
        get_service_filenames(db.db(), 2, &[hash_id]).unwrap(),
        vec![(hash_id, "two.png".to_string())]
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn directories_store_hash_maps() {
    let (db, directory) = database();
    let first = get_hash_id(db.db(), &[1]).unwrap();
    let second = get_hash_id(db.db(), &[2]).unwrap();

    set_service_directory(db.db(), 1, &[first, second], "album", 2048, "note").unwrap();

    let info = get_service_directories_info(db.db(), 1).unwrap();

    assert_eq!(info.len(), 1);
    assert_eq!(info[0].num_files, 2);
    assert_eq!(info[0].total_size, 2048);
    assert_eq!(
        get_service_directory_hashes(db.db(), 1, "album")
            .unwrap()
            .len(),
        2
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn clear_service_removes_all_rows() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[3]).unwrap();

    set_service_filename(db.db(), 1, hash_id, "one.png").unwrap();
    set_service_directory(db.db(), 1, &[hash_id], "album", 10, "").unwrap();
    clear_service(db.db(), 1).unwrap();

    assert!(get_service_filename(db.db(), 1, hash_id).unwrap().is_none());
    assert!(get_service_directories_info(db.db(), 1).unwrap().is_empty());

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
