use super::*;
use crate::db::master::get_hash_id;
use crate::db::ClientDB;

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
fn null_has_audio_reads_as_false() {
    let (db, directory) = database();
    let hash_id = get_hash_id(db.db(), &[3]).unwrap();

    db.db()
        .execute(
            "INSERT INTO files_info
             ( hash_id, size, mime, width, height, duration, num_frames, has_audio, num_words )
             VALUES ( ?, ?, ?, ?, ?, ?, ?, ?, ? )",
            &[
                &hash_id,
                &1024i64,
                &(Mime::Png.as_int() as i64),
                &640i64,
                &480i64,
                &rusqlite::types::Null,
                &rusqlite::types::Null,
                &rusqlite::types::Null,
                &rusqlite::types::Null,
            ],
        )
        .unwrap();

    let info = get_file_info(db.db(), hash_id).unwrap().unwrap();

    assert_eq!(info.size, 1024);
    assert_eq!(info.mime, Mime::Png);
    assert!(!info.has_audio);
    assert_eq!(info.width, Some(640));

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
