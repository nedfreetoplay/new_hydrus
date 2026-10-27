use super::*;
use crate::db::master::get_hash_id;
use crate::db::master_tags::get_tag_id;
use crate::db::{ClientDB, mappings};

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_tagsearch_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

fn seed(db: &DBBase) -> (i64, i64, i64) {
    let first = get_hash_id(db, &[1]).unwrap();
    let second = get_hash_id(db, &[2]).unwrap();
    let blue = get_tag_id(db, "blue eyes").unwrap();
    let blonde = get_tag_id(db, "blonde hair").unwrap();
    let creator = get_tag_id(db, "creator:someone").unwrap();

    mappings::add_mappings(db, 1, ContentStatus::Current, first, &[blue, creator]).unwrap();
    mappings::add_mappings(db, 1, ContentStatus::Current, second, &[blonde]).unwrap();

    (first, second, blue)
}

#[test]
fn tag_search_returns_matching_hashes() {
    let (db, directory) = database();
    let (first, _, _) = seed(db.db());

    let hashes = get_hash_ids_for_tag(db.db(), 1, ContentStatus::Current, "blue eyes").unwrap();

    assert_eq!(hashes, BTreeSet::from([first]));

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn namespace_search_returns_all_in_namespace() {
    let (db, directory) = database();
    let (first, _, _) = seed(db.db());

    let hashes = get_hash_ids_for_namespace(db.db(), 1, ContentStatus::Current, "creator").unwrap();

    assert_eq!(hashes, BTreeSet::from([first]));

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn wildcard_search_matches_substrings_and_namespaces() {
    let (db, directory) = database();
    let (first, second, _) = seed(db.db());

    let hashes = get_hash_ids_for_wildcard(db.db(), 1, ContentStatus::Current, "*hair*").unwrap();

    assert_eq!(hashes, BTreeSet::from([second]));

    let all = get_hash_ids_for_wildcard(db.db(), 1, ContentStatus::Current, "*e*").unwrap();

    assert!(all.contains(&first));

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn predicate_search_dispatches_by_type() {
    let (db, directory) = database();
    let (first, _, _) = seed(db.db());

    let predicate = Predicate::tag("blue eyes");
    let hashes = search_hash_ids(db.db(), 1, ContentStatus::Current, &predicate).unwrap();

    assert_eq!(hashes, BTreeSet::from([first]));

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn autocomplete_suggests_tags_and_namespaces_with_counts() {
    let (db, directory) = database();
    let (_, second, _) = seed(db.db());

    let third = get_hash_id(db.db(), &[3]).unwrap();
    let blue = get_tag_id(db.db(), "blue sky").unwrap();
    mappings::add_mappings(db.db(), 1, ContentStatus::Current, third, &[blue]).unwrap();

    let tags = suggest_tags(db.db(), 1, ContentStatus::Current, "blu", 10).unwrap();

    assert_eq!(
        tags,
        vec![
            TagSuggestion {
                text: "blue eyes".to_string(),
                count: 1
            },
            TagSuggestion {
                text: "blue sky".to_string(),
                count: 1
            }
        ]
    );

    let namespaces = suggest_namespaces(db.db(), 1, ContentStatus::Current, "cre", 10).unwrap();

    assert_eq!(
        namespaces,
        vec![TagSuggestion {
            text: "creator".to_string(),
            count: 1
        }]
    );

    assert!(
        suggest_tags(db.db(), 1, ContentStatus::Current, "zzz", 10)
            .unwrap()
            .is_empty()
    );
    assert_eq!(second, get_hash_id(db.db(), &[2]).unwrap());

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn autocomplete_escapes_sql_wildcards() {
    let (db, directory) = database();
    seed(db.db());

    assert!(
        suggest_tags(db.db(), 1, ContentStatus::Current, "%", 10)
            .unwrap()
            .is_empty()
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn missing_tables_are_empty_results() {
    let (db, directory) = database();

    assert!(
        get_hash_ids_for_tag(db.db(), 99, ContentStatus::Current, "x")
            .unwrap()
            .is_empty()
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
