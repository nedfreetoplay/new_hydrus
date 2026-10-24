use super::*;
use crate::db::ClientDB;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_physical_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn locations_round_trip() {
    let (db, directory) = database();
    let locations = vec![
        FilesStorageBaseLocation::new("C:/hydrus/a", 3, None),
        FilesStorageBaseLocation::new("C:/hydrus/b", 7, Some(1024)),
    ];

    set_current_locations(db.db(), &locations).unwrap();
    set_ideal_locations(db.db(), &locations).unwrap();

    assert_eq!(get_current_locations(db.db()).unwrap(), locations);
    assert_eq!(get_ideal_locations(db.db()).unwrap(), locations);

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn subfolders_resolve_base_locations() {
    let (db, directory) = database();
    let location = FilesStorageBaseLocation::new("C:/hydrus/a", 1, None);
    let subfolders = vec![FilesStorageSubfolder::new("fa", location.clone())];

    set_subfolders(db.db(), &subfolders).unwrap();

    let loaded = get_subfolders(db.db(), &[location]).unwrap();

    assert_eq!(loaded, subfolders);

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn granularity_and_thumbnail_override_round_trip() {
    let (db, directory) = database();

    set_storage_granularity(db.db(), 5).unwrap();
    set_ideal_thumbnail_override_location(db.db(), Some("C:/thumbs")).unwrap();

    assert_eq!(get_storage_granularity(db.db()).unwrap(), Some(5));
    assert_eq!(
        get_ideal_thumbnail_override_location(db.db())
            .unwrap()
            .as_deref(),
        Some("C:/thumbs")
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
