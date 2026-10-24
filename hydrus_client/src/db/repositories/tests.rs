
use super::*;
use crate::db::ClientDB;
use crate::db::master::get_hash_id;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_repos_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn remote_thumbnails_round_trip() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[1]).unwrap();

    assert!(!has_remote_thumbnail(db.db(), 1, hash_id).unwrap());

    set_remote_thumbnail(db.db(), 1, hash_id, b"thumb").unwrap();

    assert!(has_remote_thumbnail(db.db(), 1, hash_id).unwrap());
    assert_eq!(
        get_remote_thumbnail(db.db(), 1, hash_id).unwrap(),
        Some(b"thumb".to_vec())
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn updates_are_processed_in_order() {
    let (db, directory) = database();

    add_update(db.db(), 1, 2, &[2], 20, 30).unwrap();
    add_update(db.db(), 1, 1, &[1], 10, 20).unwrap();

    let updates = get_updates(db.db(), 1).unwrap();

    assert_eq!(updates.len(), 2);
    assert_eq!(updates[0].update_index, 1);
    assert_eq!(
        get_next_unprocessed_update(db.db(), 1)
            .unwrap()
            .unwrap()
            .update_index,
        1
    );

    mark_update_processed(db.db(), 1, 1).unwrap();

    assert_eq!(
        get_next_unprocessed_update(db.db(), 1)
            .unwrap()
            .unwrap()
            .update_index,
        2
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn updates_can_be_cleared() {
    let (db, directory) = database();
    add_update(db.db(), 1, 1, &[1], 0, 1).unwrap();

    delete_updates(db.db(), 1).unwrap();

    assert!(get_updates(db.db(), 1).unwrap().is_empty());

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
