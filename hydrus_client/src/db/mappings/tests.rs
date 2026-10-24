use super::*;
use crate::db::ClientDB;
use crate::db::master::get_hash_id;
use crate::db::master_tags::get_tag_id;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_mappings_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn mappings_round_trip_both_ways() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[1]).unwrap();
    let tag_id = get_tag_id(db.db(), "blue eyes").unwrap();

    add_mappings(db.db(), 1, ContentStatus::Current, hash_id, &[tag_id]).unwrap();

    assert_eq!(
        get_tag_ids(db.db(), 1, ContentStatus::Current, hash_id).unwrap(),
        vec![tag_id]
    );
    assert_eq!(
        get_hash_ids(db.db(), 1, ContentStatus::Current, tag_id).unwrap(),
        vec![hash_id]
    );
    assert_eq!(
        get_tags(db.db(), 1, ContentStatus::Current, hash_id).unwrap(),
        vec!["blue eyes".to_string()]
    );
    assert_eq!(
        get_num_mappings(db.db(), 1, ContentStatus::Current).unwrap(),
        1
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn statuses_and_services_are_independent() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[2]).unwrap();
    let tag_id = get_tag_id(db.db(), "tag").unwrap();

    add_mappings(db.db(), 1, ContentStatus::Current, hash_id, &[tag_id]).unwrap();
    add_mappings(db.db(), 1, ContentStatus::Deleted, hash_id, &[tag_id]).unwrap();
    add_mappings(db.db(), 2, ContentStatus::Current, hash_id, &[tag_id]).unwrap();

    assert_eq!(
        get_tag_ids(db.db(), 1, ContentStatus::Current, hash_id)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        get_tag_ids(db.db(), 1, ContentStatus::Deleted, hash_id)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        get_tag_ids(db.db(), 2, ContentStatus::Current, hash_id)
            .unwrap()
            .len(),
        1
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn removing_mappings_deletes_rows() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[3]).unwrap();
    let tag_id = get_tag_id(db.db(), "tag").unwrap();

    add_mappings(db.db(), 1, ContentStatus::Current, hash_id, &[tag_id]).unwrap();
    remove_mappings(db.db(), 1, ContentStatus::Current, hash_id, &[tag_id]).unwrap();

    assert!(
        get_tag_ids(db.db(), 1, ContentStatus::Current, hash_id)
            .unwrap()
            .is_empty()
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
