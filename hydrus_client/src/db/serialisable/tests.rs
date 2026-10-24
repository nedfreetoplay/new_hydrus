
use super::*;
use crate::db::ClientDB;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_serialisable_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn simple_json_round_trips() {
    let (db, directory) = database();
    let value = serde_json::json!({"a": 1});

    set_json_simple(db.db(), "simple", &value).unwrap();

    assert_eq!(get_json_simple(db.db(), "simple").unwrap(), Some(value));

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn named_dumps_round_trip_and_list() {
    let (db, directory) = database();

    set_json_dump_named(db.db(), 22, "options", 8, 100, "{}").unwrap();
    set_json_dump_named(db.db(), 22, "backup", 8, 200, "{}").unwrap();

    assert_eq!(
        get_json_dump_names(db.db(), 22).unwrap(),
        vec!["backup", "options"]
    );
    assert_eq!(
        get_json_dump_named(db.db(), 22, "options").unwrap(),
        Some((8, 100, "{}".to_string()))
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hashed_dumps_round_trip() {
    let (db, directory) = database();

    set_json_dump_hashed(db.db(), &[1, 2], 22, 8, "{}").unwrap();

    assert_eq!(
        get_json_dump_hashed(db.db(), &[1, 2], 22).unwrap(),
        Some((8, "{}".to_string()))
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
