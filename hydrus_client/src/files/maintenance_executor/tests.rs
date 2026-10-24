
use super::*;
use crate::db::ClientDB;
use crate::db::master::get_hash_id;
use crate::files::{FilesStorageBaseLocation, FilesStorageSubfolder};
use hydrus_core::hydrus_constants::Mime;
use image::{Rgba, RgbaImage};

fn setup() -> (
    ClientDB,
    FilesMaintenanceExecutor,
    std::path::PathBuf,
    i64,
    Vec<u8>,
) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_maint_exec_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let db = ClientDB::new(&directory).unwrap();
    let hash = vec![0xab, 0xcd];
    let hash_id = get_hash_id(db.db(), &hash).unwrap();

    let base = FilesStorageBaseLocation::new(directory.join("files"), 1, None);
    let thumbs = FilesStorageBaseLocation::new(directory.join("thumbs"), 1, None);
    let manager = ClientFilesManager::new(vec![
        FilesStorageSubfolder::new("f", base),
        FilesStorageSubfolder::new("t", thumbs),
    ]);
    manager.make_sure_exists().unwrap();

    let path = manager.get_file_path(&hash, Mime::Png, false).unwrap();
    let image = RgbaImage::from_pixel(8, 8, Rgba([10, 20, 30, 255]));
    image.save(&path).unwrap();

    let info = core_files::get_file_info(&path, Some(Mime::Png), false).unwrap();
    files_metadata::set_file_info(db.db(), hash_id, &info).unwrap();

    (
        db,
        FilesMaintenanceExecutor::new(manager),
        directory,
        hash_id,
        hash,
    )
}

#[test]
fn metadata_and_thumbnail_jobs_run() {
    let (db, executor, directory, hash_id, hash) = setup();

    assert!(
        executor
            .run_job(db.db(), hash_id, REGENERATE_FILE_DATA_JOB_FILE_METADATA)
            .unwrap()
    );
    assert!(
        executor
            .run_job(db.db(), hash_id, REGENERATE_FILE_DATA_JOB_FORCE_THUMBNAIL)
            .unwrap()
    );

    let info = files_metadata::get_file_info(db.db(), hash_id)
        .unwrap()
        .unwrap();
    assert_eq!(info.width, Some(8));

    let thumbnail_path = executor.files_manager.get_thumbnail_path(&hash).unwrap();
    assert!(thumbnail_path.exists());

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn blurhash_and_transparency_jobs_run() {
    let (db, executor, directory, hash_id, _) = setup();

    assert!(
        executor
            .run_job(db.db(), hash_id, REGENERATE_FILE_DATA_JOB_BLURHASH)
            .unwrap()
    );
    assert!(
        executor
            .run_job(
                db.db(),
                hash_id,
                REGENERATE_FILE_DATA_JOB_FILE_HAS_TRANSPARENCY
            )
            .unwrap()
    );

    assert!(
        files_metadata::get_blurhash(db.db(), hash_id)
            .unwrap()
            .is_some()
    );
    assert!(
        !files_metadata::has_metadata_flag(
            db.db(),
            hash_id,
            files_metadata::MetadataFlag::Transparency
        )
        .unwrap()
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn unsupported_jobs_return_false() {
    let (db, executor, directory, hash_id, _) = setup();

    assert!(
        !executor
            .run_job(db.db(), hash_id, REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS)
            .unwrap()
    );

    drop(db);
    let _ = std::fs::remove_dir_all(directory);
}
