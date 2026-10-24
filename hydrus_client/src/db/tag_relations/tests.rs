use super::*;
use crate::db::ClientDB;
use crate::db::master_tags::get_tag_id;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_tagrelations_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn sibling_pairs_round_trip() {
    let (db, directory) = database();
    let bad = get_tag_id(db.db(), "blue eyes").unwrap();
    let good = get_tag_id(db.db(), "eyes:blue").unwrap();

    add_pair(
        db.db(),
        TagRelation::Siblings,
        1,
        ContentStatus::Current,
        bad,
        good,
        None,
    )
    .unwrap();

    assert_eq!(
        get_pairs(db.db(), TagRelation::Siblings, 1, ContentStatus::Current).unwrap(),
        vec![(bad, good)]
    );
    assert_eq!(
        get_pairs_with_tags(db.db(), TagRelation::Siblings, 1, ContentStatus::Current).unwrap(),
        vec![("blue eyes".to_string(), "eyes:blue".to_string())]
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn sibling_chains_resolve_transitively() {
    let (db, directory) = database();
    let a = get_tag_id(db.db(), "a").unwrap();
    let b = get_tag_id(db.db(), "b").unwrap();
    let c = get_tag_id(db.db(), "c").unwrap();

    for (bad, good) in [(a, b), (b, c)] {
        add_pair(
            db.db(),
            TagRelation::Siblings,
            1,
            ContentStatus::Current,
            bad,
            good,
            None,
        )
        .unwrap();
    }

    assert_eq!(resolve_siblings(db.db(), 1, "a").unwrap(), "c");
    assert_eq!(resolve_siblings(db.db(), 1, "unknown").unwrap(), "unknown");

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn parent_chains_return_all_ancestors() {
    let (db, directory) = database();
    let child = get_tag_id(db.db(), "child").unwrap();
    let parent = get_tag_id(db.db(), "parent").unwrap();
    let grandparent = get_tag_id(db.db(), "grandparent").unwrap();

    for (a, b) in [(child, parent), (parent, grandparent)] {
        add_pair(
            db.db(),
            TagRelation::Parents,
            1,
            ContentStatus::Current,
            a,
            b,
            None,
        )
        .unwrap();
    }

    let ancestors = get_ancestors(db.db(), 1, "child").unwrap();

    assert_eq!(
        ancestors,
        BTreeSet::from(["parent".to_string(), "grandparent".to_string()])
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn removing_pairs_works() {
    let (db, directory) = database();
    let a = get_tag_id(db.db(), "a").unwrap();
    let b = get_tag_id(db.db(), "b").unwrap();

    add_pair(
        db.db(),
        TagRelation::Siblings,
        1,
        ContentStatus::Current,
        a,
        b,
        None,
    )
    .unwrap();
    remove_pair(
        db.db(),
        TagRelation::Siblings,
        1,
        ContentStatus::Current,
        a,
        b,
    )
    .unwrap();

    assert!(
        get_pairs(db.db(), TagRelation::Siblings, 1, ContentStatus::Current)
            .unwrap()
            .is_empty()
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
