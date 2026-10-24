
use super::*;
use crate::importing::options::FileFilteringImportOptions;
use hydrus_core::hydrus_exceptions::unknown;
use image::{Rgba, RgbaImage};

fn png_file(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "hydrus_client_import_{name}_{}_{}.png",
        std::process::id(),
        rand::random::<u64>()
    ));
    RgbaImage::from_pixel(8, 8, Rgba([1, 2, 3, 255]))
        .save(&path)
        .unwrap();
    path
}

#[test]
fn unknown_and_deleted_statuses_control_import() {
    let options = ImportOptionsContainer::default();

    assert!(FileImportStatus::unknown().should_import(&options));

    let deleted = FileImportStatus::new(STATUS_DELETED, None, None, "");
    assert!(!deleted.should_import(&options));

    let mut allow_deleted = options.clone();
    allow_deleted.file_filtering.excludes_deleted = false;
    assert!(deleted.should_import(&allow_deleted));
}

#[test]
fn job_hashes_and_reads_file_info() {
    let path = png_file("info");
    let mut job = FileImportJob::new(&path, ImportOptionsContainer::default(), None);

    let status = job.do_work(|_| Ok(false)).unwrap();

    assert_eq!(status.status, STATUS_UNKNOWN);
    assert_eq!(status.mime, Some(Mime::Png));
    assert!(job.get_hash().is_some());
    assert_eq!(job.get_file_info().unwrap().width, Some(8));
    assert!(job.get_file_modified_timestamp_ms().is_some());

    let _ = std::fs::remove_file(path);
}

#[test]
fn known_hashes_short_circuit_as_redundant() {
    let path = png_file("known");
    let mut job = FileImportJob::new(&path, ImportOptionsContainer::default(), None);

    let status = job.do_work(|_| Ok(true)).unwrap();

    assert!(status.already_in_db());
    assert!(job.get_file_info().is_none());

    let _ = std::fs::remove_file(path);
}

#[test]
fn filters_reject_files_with_status_error() {
    let path = png_file("filtered");
    let options = ImportOptionsContainer {
        file_filtering: FileFilteringImportOptions {
            allowed_mimes: vec![Mime::Jpeg],
            ..Default::default()
        },
        ..Default::default()
    };
    let mut job = FileImportJob::new(&path, options, None);

    let status = job.do_work(|_| Ok(false)).unwrap();

    assert_eq!(status.status, STATUS_ERROR);
    assert!(status.note.contains("file filter"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn db_lookup_errors_propagate() {
    let path = png_file("error");
    let mut job = FileImportJob::new(&path, ImportOptionsContainer::default(), None);

    assert!(job.do_work(|_| Err(unknown("db unavailable"))).is_err());

    let _ = std::fs::remove_file(path);
}
