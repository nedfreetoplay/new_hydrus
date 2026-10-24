
use super::*;
use crate::db::ClientDB;
use crate::db::master::get_hash_id;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_filestorage_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn current_files_round_trip() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[1, 2, 3]).unwrap();

    add_files(db.db(), 1, ContentStatus::Current, &[hash_id], Some(100)).unwrap();

    assert_eq!(
        get_hash_ids(db.db(), 1, ContentStatus::Current).unwrap(),
        vec![hash_id]
    );
    assert_eq!(
        get_hashes(db.db(), 1, ContentStatus::Current).unwrap(),
        vec![vec![1, 2, 3]]
    );

    remove_files(db.db(), 1, ContentStatus::Current, &[hash_id]).unwrap();

    assert!(
        get_hash_ids(db.db(), 1, ContentStatus::Current)
            .unwrap()
            .is_empty()
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn inbox_is_tracked_separately() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[4, 5, 6]).unwrap();

    add_files(db.db(), 1, ContentStatus::Current, &[hash_id], None).unwrap();
    set_inbox(db.db(), &[hash_id], true).unwrap();

    assert!(is_inbox(db.db(), hash_id).unwrap());
    assert_eq!(get_num_inbox(db.db()).unwrap(), 1);
    assert_eq!(get_num_archive(db.db(), 1).unwrap(), 0);

    set_inbox(db.db(), &[hash_id], false).unwrap();

    assert!(!is_inbox(db.db(), hash_id).unwrap());

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn tables_are_per_service() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[7]).unwrap();

    add_files(db.db(), 1, ContentStatus::Current, &[hash_id], None).unwrap();
    add_files(db.db(), 2, ContentStatus::Deleted, &[hash_id], None).unwrap();

    assert_eq!(
        get_hash_ids(db.db(), 1, ContentStatus::Current)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        get_hash_ids(db.db(), 2, ContentStatus::Deleted)
            .unwrap()
            .len(),
        1
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
