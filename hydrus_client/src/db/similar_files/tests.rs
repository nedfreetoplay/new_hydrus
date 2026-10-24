
use super::*;
use crate::db::ClientDB;
use crate::db::master::get_hash_id;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_similar_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn perceptual_hashes_round_trip() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[1]).unwrap();

    set_perceptual_hashes(db.db(), hash_id, &[vec![0xaa, 0xbb]]).unwrap();

    assert_eq!(
        get_perceptual_hashes(db.db(), hash_id).unwrap(),
        vec![vec![0xaa, 0xbb]]
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn similar_search_respects_distance() {
    let (db, directory) = database();
    let first = get_hash_id(db.db(), &[1]).unwrap();
    let second = get_hash_id(db.db(), &[2]).unwrap();
    let third = get_hash_id(db.db(), &[3]).unwrap();

    set_perceptual_hashes(db.db(), first, &[vec![0, 0, 0, 0, 0, 0, 0, 0]]).unwrap();
    set_perceptual_hashes(db.db(), second, &[vec![0, 0, 0, 0, 0, 0, 0, 1]]).unwrap();
    set_perceptual_hashes(db.db(), third, &[vec![255, 0, 0, 0, 0, 0, 0, 0]]).unwrap();

    let matches = search_similar(db.db(), &[0, 0, 0, 0, 0, 0, 0, 0], 2, Some(first)).unwrap();

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].hash_id, second);
    assert_eq!(matches[0].distance, 1);

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn pixel_hashes_are_indexed() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[1]).unwrap();

    set_pixel_hash(db.db(), hash_id, &[9, 9]).unwrap();

    assert_eq!(get_pixel_hash(db.db(), hash_id).unwrap(), Some(vec![9, 9]));
    assert_eq!(
        get_hash_ids_for_pixel_hash(db.db(), &[9, 9]).unwrap(),
        vec![hash_id]
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn search_distance_is_cached() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[1]).unwrap();

    assert_eq!(get_search_distance(db.db(), hash_id).unwrap(), None);

    set_search_distance(db.db(), hash_id, 4).unwrap();

    assert_eq!(get_search_distance(db.db(), hash_id).unwrap(), Some(4));

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
