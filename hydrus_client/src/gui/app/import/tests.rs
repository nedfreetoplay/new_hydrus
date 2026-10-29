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

fn set_local_import_options(
    controller: &Controller,
    file_filtering: Option<crate::importing::FileFilteringImportOptions>,
    locations: Option<crate::importing::LocationImportOptions>,
) {
    let mut manager = super::super::import_options::load(controller);
    let mut local = ImportOptionsContainer::default();
    local.file_filtering = file_filtering;
    local.locations = locations;
    manager.set_default("local_import", local);
    super::super::import_options::save_manager(controller, &manager);
}

#[test]
fn local_import_options_filter_files_and_archive_them() {
    let (controller, directory) = temporary_controller();
    let source = directory.join("source.png");

    image::RgbaImage::from_pixel(8, 6, image::Rgba([10, 20, 30, 255]))
        .save(&source)
        .unwrap();

    set_local_import_options(
        &controller,
        Some(crate::importing::FileFilteringImportOptions {
            max_size: Some(1),
            ..Default::default()
        }),
        None,
    );

    let summary = import_paths(&controller, &[source.clone()]);
    assert_eq!(summary.imported, 0);
    assert!(
        summary.errors[0].contains("file filter"),
        "{:?}",
        summary.errors
    );

    set_local_import_options(
        &controller,
        None,
        Some(crate::importing::LocationImportOptions {
            automatically_archives: true,
            ..Default::default()
        }),
    );

    let summary = import_paths(&controller, &[source]);
    assert_eq!(summary.imported, 1, "{:?}", summary.errors);

    let hash_id = query::hash_id_for(&controller, &summary.hashes[0]).unwrap();
    assert!(!crate::db::files_storage::is_inbox(controller.db().db(), hash_id).unwrap());

    drop(controller);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn local_import_options_can_target_another_file_domain() {
    let (controller, directory) = temporary_controller();
    let source = directory.join("source.png");

    image::RgbaImage::from_pixel(8, 6, image::Rgba([10, 20, 30, 255]))
        .save(&source)
        .unwrap();

    let archive_id = controller
        .add_service("archive", ServiceType::LocalFileDomain)
        .unwrap();

    set_local_import_options(
        &controller,
        None,
        Some(crate::importing::LocationImportOptions {
            destination_services: vec!["archive".to_string()],
            ..Default::default()
        }),
    );

    let summary = import_paths(&controller, &[source]);
    assert_eq!(summary.imported, 1, "{:?}", summary.errors);

    let hash_id = query::hash_id_for(&controller, &summary.hashes[0]).unwrap();
    let current = crate::db::files_storage::get_hash_ids(
        controller.db().db(),
        archive_id,
        ContentStatus::Current,
    )
    .unwrap();
    assert!(current.contains(&hash_id));

    let my_files_id = query::local_file_service_id(&controller, "my files").unwrap();
    let my_files = crate::db::files_storage::get_hash_ids(
        controller.db().db(),
        my_files_id,
        ContentStatus::Current,
    )
    .unwrap();
    assert!(!my_files.contains(&hash_id));

    drop(controller);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn import_options_manager_round_trips_through_client_options() {
    let (controller, directory) = temporary_controller();
    let mut manager = super::super::import_options::load(&controller);

    manager.add_favourite("saved", ImportOptionsContainer::with_defaults());
    super::super::import_options::save_manager(&controller, &manager);

    let loaded = super::super::import_options::load(&controller);
    assert!(loaded.get_favourite("saved").is_some());

    drop(controller);
    let _ = std::fs::remove_dir_all(directory);
}
