use super::*;
use crate::db::ClientDB;
use crate::db::master_tags::get_tag_id;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_counts_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn counts_accumulate() {
    let (db, directory) = database();
    let tag_id = get_tag_id(db.db(), "blue eyes").unwrap();

    add_counts(db.db(), 1, 2, tag_id, 3, 1).unwrap();
    add_counts(db.db(), 1, 2, tag_id, 2, -1).unwrap();

    assert_eq!(get_counts(db.db(), 1, 2, tag_id).unwrap(), (5, 0));
    assert_eq!(get_total_current_count(db.db(), 1, 2).unwrap(), 5);

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn counts_are_per_service_pair() {
    let (db, directory) = database();
    let tag_id = get_tag_id(db.db(), "tag").unwrap();

    set_counts(db.db(), 1, 1, tag_id, 10, 2).unwrap();
    set_counts(db.db(), 1, 2, tag_id, 20, 4).unwrap();

    assert_eq!(get_counts(db.db(), 1, 1, tag_id).unwrap(), (10, 2));
    assert_eq!(get_counts(db.db(), 1, 2, tag_id).unwrap(), (20, 4));

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn clearing_can_keep_current_counts() {
    let (db, directory) = database();
    let tag_id = get_tag_id(db.db(), "tag").unwrap();

    set_counts(db.db(), 1, 1, tag_id, 10, 2).unwrap();
    clear_counts(db.db(), 1, 1, true, false).unwrap();

    assert_eq!(get_counts(db.db(), 1, 1, tag_id).unwrap(), (10, 0));

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn in_memory_cache_accumulates_and_tracks_dirty() {
    let mut cache = MappingCountsCache::new();

    cache.add(1, 2, 3, 5, 1);
    cache.add(1, 2, 3, -2, 4);
    cache.add(1, 2, 4, 1, 0);

    assert_eq!(cache.get(1, 2, 3), (3, 5));
    assert_eq!(cache.len(), 2);
    assert!(cache.is_dirty());

    cache.set_clean();
    assert!(!cache.is_dirty());

    cache.remove(1, 2, 3);
    assert_eq!(cache.get(1, 2, 3), (0, 0));
    assert!(cache.is_dirty());
}

#[test]
fn cache_clear_empties_entries() {
    let mut cache = MappingCountsCache::new();
    cache.set(1, 1, 1, 2, 3);
    cache.clear();

    assert!(cache.is_empty());
}
