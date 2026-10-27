use super::*;
use crate::db::{ClientDB, master_tags};

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_query_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn multi_word_tags_stay_one_predicate() {
    let (db, directory) = database();

    master_tags::get_tag_id(db.db(), "blue eyes").unwrap();
    master_tags::get_tag_id(db.db(), "sky").unwrap();

    let plan = search::plan(db.db(), "blue eyes -sky system:inbox").unwrap();
    let terms: Vec<&str> = plan.tags.iter().map(|term| term.text.as_str()).collect();

    assert_eq!(terms, vec!["blue eyes", "sky"]);
    assert!(!plan.tags[1].inclusive);
    assert_eq!(plan.predicates.len(), 1);

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn unknown_phrases_fall_back_to_separate_words() {
    let (db, directory) = database();

    master_tags::get_tag_id(db.db(), "blue eyes").unwrap();

    let plan = search::plan(db.db(), "blue eyes mystery").unwrap();
    let terms: Vec<&str> = plan.tags.iter().map(|term| term.text.as_str()).collect();

    assert_eq!(terms, vec!["blue eyes", "mystery"]);

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hex_round_trips_hashes() {
    let hash = vec![0x00, 0xab, 0xff, 0x10];

    assert_eq!(unhex(&hex(&hash)), Some(hash));
    assert_eq!(unhex("xyz"), None);
}
