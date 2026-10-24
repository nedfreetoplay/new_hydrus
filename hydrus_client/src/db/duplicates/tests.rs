use super::*;
use crate::db::ClientDB;
use crate::db::master::get_hash_id;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_duplicates_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn potential_pairs_round_trip_and_filter() {
    let (db, directory) = database();
    let first = get_hash_id(db.db(), &[1]).unwrap();
    let second = get_hash_id(db.db(), &[2]).unwrap();

    add_potential_pair(db.db(), first, second, 4).unwrap();
    add_potential_pair(db.db(), second, first + 100, 10).unwrap();

    assert_eq!(
        get_potential_pairs(db.db(), Some(5)).unwrap(),
        vec![(first, second, 4)]
    );

    remove_potential_pair(db.db(), first, second).unwrap();

    assert_eq!(get_potential_pairs(db.db(), None).unwrap().len(), 1);

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn false_positives_are_recorded() {
    let (db, directory) = database();
    let first = get_hash_id(db.db(), &[1]).unwrap();
    let second = get_hash_id(db.db(), &[2]).unwrap();

    assert!(!is_false_positive(db.db(), first, second).unwrap());

    add_false_positive(db.db(), first, second).unwrap();

    assert!(is_false_positive(db.db(), first, second).unwrap());

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn alternate_groups_store_members() {
    let (db, directory) = database();
    let first = get_hash_id(db.db(), &[1]).unwrap();
    let second = get_hash_id(db.db(), &[2]).unwrap();

    let group = create_alternate_group(db.db(), &[first]).unwrap();
    add_to_alternate_group(db.db(), group, second).unwrap();

    assert_eq!(
        get_alternate_group_members(db.db(), group).unwrap(),
        vec![first, second]
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn duplicate_groups_have_a_king() {
    let (db, directory) = database();
    let king = get_hash_id(db.db(), &[1]).unwrap();
    let member = get_hash_id(db.db(), &[2]).unwrap();

    set_duplicate_group(db.db(), 1, king, &[king, member]).unwrap();

    assert_eq!(get_duplicate_king(db.db(), 1).unwrap(), Some(king));
    assert_eq!(
        get_duplicate_group_members(db.db(), 1).unwrap(),
        vec![king, member]
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
