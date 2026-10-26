use std::path::PathBuf;

use super::*;
use crate::controller::Controller;
use crate::gui::app::query;

fn temporary_controller() -> (Controller, PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_client_import_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));

    let _ = crate::client_options::register_serialisable_type();

    let controller = Controller::new(&directory).unwrap();
    controller.init_model().unwrap();

    (controller, directory)
}

#[test]
fn imports_a_png_and_writes_a_thumbnail() {
    let (controller, directory) = temporary_controller();
    let source = directory.join("source.png");

    image::RgbaImage::from_pixel(8, 6, image::Rgba([10, 20, 30, 255]))
        .save(&source)
        .unwrap();

    let summary = import_paths(&controller, &[source]);

    assert_eq!(summary.imported, 1, "{:?}", summary.errors);
    assert_eq!(summary.errors, Vec::<String>::new());

    let hash = &summary.hashes[0];
    let hash_id = query::hash_id_for(&controller, hash).expect("hash is registered");
    let info = query::file_info_for(&controller, hash).expect("file info is registered");

    assert_eq!(info.width, Some(8));
    assert_eq!(info.height, Some(6));

    let service_id = query::local_file_service_id(&controller, "my files").unwrap();
    let current = crate::db::files_storage::get_hash_ids(
        controller.db().db(),
        service_id,
        ContentStatus::Current,
    )
    .unwrap();

    assert!(current.contains(&hash_id));
    assert!(crate::db::files_storage::is_inbox(controller.db().db(), hash_id).unwrap());

    let thumbnail = controller.files_manager().get_thumbnail_path(hash).unwrap();
    assert!(thumbnail.exists(), "thumbnail was not generated");

    drop(controller);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn unknown_files_are_skipped_without_panicking() {
    let (controller, directory) = temporary_controller();
    let source = directory.join("source.bin");
    std::fs::write(&source, b"this is not an image").unwrap();

    let summary = import_paths(&controller, &[source]);

    assert_eq!(summary.imported, 0);
    assert_eq!(summary.skipped, 1);
    assert!(!summary.errors.is_empty());

    drop(controller);
    let _ = std::fs::remove_dir_all(directory);
}
