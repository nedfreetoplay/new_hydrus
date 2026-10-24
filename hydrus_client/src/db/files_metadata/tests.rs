use super::*;
use crate::db::ClientDB;
use crate::db::master::get_hash_id;

fn database() -> (ClientDB, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_fileflags_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    (ClientDB::new(&directory).unwrap(), directory)
}

#[test]
fn metadata_flags_round_trip() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[1]).unwrap();

    assert!(!has_metadata_flag(db.db(), hash_id, MetadataFlag::Exif).unwrap());

    set_metadata_flag(db.db(), hash_id, MetadataFlag::Exif, true).unwrap();
    set_metadata_flag(db.db(), hash_id, MetadataFlag::Transparency, true).unwrap();

    assert!(has_metadata_flag(db.db(), hash_id, MetadataFlag::Exif).unwrap());
    assert_eq!(
        get_hash_ids_with_flag(db.db(), MetadataFlag::Transparency).unwrap(),
        vec![hash_id]
    );

    set_metadata_flag(db.db(), hash_id, MetadataFlag::Exif, false).unwrap();
    assert!(!has_metadata_flag(db.db(), hash_id, MetadataFlag::Exif).unwrap());

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn forced_mime_round_trips() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[2]).unwrap();

    set_forced_mime(db.db(), hash_id, Mime::Png).unwrap();

    assert_eq!(get_forced_mime(db.db(), hash_id).unwrap(), Some(Mime::Png));

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
