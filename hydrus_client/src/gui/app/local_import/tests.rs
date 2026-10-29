use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use serde_json::Value;

use super::*;
use crate::gui::app::model::AppState;
use crate::importing::FilenameTaggingOptions;

fn temp_dir(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_local_import_{label}_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

fn png_file(directory: &Path, name: &str) -> PathBuf {
    let path = directory.join(name);
    RgbaImage::from_pixel(8, 8, Rgba([1, 2, 3, 255]))
        .save(&path)
        .unwrap();
    path
}

fn state(directory: &Path) -> AppState {
    let _ = crate::client_options::register_serialisable_type();

    let controller = crate::controller::Controller::new(directory).unwrap();
    controller.init_model().unwrap();

    AppState::new(std::sync::Arc::new(controller))
}

#[test]
fn parses_good_bad_and_sidecar_files() {
    let directory = temp_dir("parse");
    let good = png_file(&directory, "image.png");
    let sidecar = directory.join("image.txt");
    std::fs::write(&sidecar, "tags").unwrap();
    let empty = directory.join("empty.png");
    std::fs::File::create(&empty).unwrap();
    let thumbs = directory.join("Thumbs.db");
    std::fs::write(&thumbs, "not an image").unwrap();
    let missing = directory.join("missing.png");

    let mut review = LocalImportReview::default();
    review.add_paths(&[good.clone(), sidecar.clone(), empty, thumbs, missing]);

    let counts = review.counts();

    assert_eq!(counts.total, 5);
    assert_eq!(counts.good, 1);
    assert_eq!(counts.sidecar, 1);
    assert_eq!(counts.empty, 1);
    assert_eq!(counts.unimportable, 1);
    assert_eq!(counts.missing, 1);
    assert_eq!(counts.bad, 3);

    let good_parse = review
        .paths
        .iter()
        .find(|parse| parse.path == good)
        .unwrap();
    assert_eq!(good_parse.result, LocalFileResult::Good);
    assert_eq!(good_parse.pretty_mime(), "png");
    assert!(good_parse.size > 0);

    assert_eq!(review.good_paths(), vec![good]);
    assert_eq!(review.sidecar_paths(), vec![sidecar]);

    let summary = review.summary();
    assert!(summary.contains("5 files parsed"));
    assert!(summary.contains("1 good | 3 bad"));
    assert!(summary.contains("1 were missing"));
    assert!(summary.contains("txt/json/xml sidecars"));

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn expands_folders_and_respects_search_subdirectories() {
    let directory = temp_dir("folders");
    let root_file = png_file(&directory, "root.png");
    let subdirectory = directory.join("sub");
    std::fs::create_dir_all(&subdirectory).unwrap();
    let nested_file = png_file(&subdirectory, "nested.png");

    let mut review = LocalImportReview::default();
    review.add_paths(&[directory.clone()]);
    assert_eq!(review.paths.len(), 2);
    assert!(review.paths.iter().any(|parse| parse.path == root_file));
    assert!(review.paths.iter().any(|parse| parse.path == nested_file));

    review.remove_paths(&[nested_file.clone()]);
    review.search_subdirectories = false;
    review.add_paths(&[directory.clone()]);
    assert_eq!(review.paths.len(), 1);
    assert_eq!(review.paths[0].path, root_file);

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn duplicate_paths_are_ignored_and_indices_are_contiguous() {
    let directory = temp_dir("dupes");
    let first = png_file(&directory, "a.png");
    let second = png_file(&directory, "b.png");

    let mut review = LocalImportReview::default();
    review.add_paths(&[first.clone(), second.clone(), first.clone()]);

    assert_eq!(review.paths.len(), 2);
    assert_eq!(review.paths[0].index, 1);
    assert_eq!(review.paths[1].index, 2);

    let removed = review.remove_paths(&[first]);
    assert_eq!(removed, 1);
    assert_eq!(review.paths.len(), 1);
    assert_eq!(review.paths[0].index, 1);

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn dialog_counts_and_can_import_flag_are_rendered() {
    let directory = temp_dir("dialog");
    let good = png_file(&directory, "ok.png");
    let missing = directory.join("gone.png");
    let mut app = state(&directory);

    app.local_import_review.add_paths(&[good, missing]);
    let value: Value = serde_json::from_str(&dialog_json(&app)).unwrap();

    assert_eq!(value["counts"]["good"], 1);
    assert_eq!(value["counts"]["missing"], 1);
    assert_eq!(value["can_import"], true);
    assert_eq!(value["good_count"], 1);
    assert_eq!(value["paths"].as_array().unwrap().len(), 2);
    assert!(
        value["summary"]
            .as_str()
            .unwrap()
            .contains("2 files parsed")
    );

    drop(app);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn tagging_save_keeps_only_configured_rules() {
    let directory = temp_dir("tagging");
    let good = png_file(&directory, "tagged.png");
    let mut app = state(&directory);
    app.local_import_review.add_paths(&[good]);

    let options = FilenameTaggingOptions {
        tags_for_all: vec!["blue eyes".to_string()],
        add_filename: true,
        ..Default::default()
    };
    let payload = serde_json::json!({
        "filename_tagging": {
            "abcd": options,
            "ef01": FilenameTaggingOptions::default(),
        },
        "import_after_save": false,
    })
    .to_string();

    let commands = tagging_save(&mut app, &payload);

    assert!(matches!(
        commands.first(),
        Some(crate::gui::app::UiCommand::LocalImportReviewDialog(_))
    ));
    assert_eq!(app.local_import_review.filename_tagging.len(), 1);
    let rules = app
        .local_import_review
        .filename_tagging
        .get("abcd")
        .unwrap();
    assert_eq!(rules.tags_for_all, vec!["blue eyes".to_string()]);
    assert!(rules.add_filename);

    drop(app);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn importing_without_good_paths_is_refused() {
    let directory = temp_dir("empty_import");
    let mut app = state(&directory);

    let commands = import_now(&mut app);

    assert!(matches!(
        commands.first(),
        Some(crate::gui::app::UiCommand::NotifyMessage(message))
            if message.contains("no good files")
    ));

    drop(app);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn importing_good_files_can_delete_the_originals() {
    use std::time::{Duration, Instant};

    let directory = temp_dir("real_import");
    let good = png_file(&directory, "import-me.png");
    let mut app = state(&directory);
    app.local_import_review.add_paths(&[good.clone()]);
    app.local_import_review.delete_after_success = true;

    let commands = import_now(&mut app);

    assert!(matches!(
        commands.first(),
        Some(crate::gui::app::UiCommand::CloseLocalImportReview)
    ));

    let deadline = Instant::now() + Duration::from_secs(20);

    while Instant::now() < deadline && app.import_queue.running {
        let _ = crate::gui::app::download::tick(&mut app);
        std::thread::sleep(Duration::from_millis(50));
    }

    assert!(!good.exists(), "the source file should be deleted");
    assert_eq!(app.import_queue.imported, 1);

    drop(app);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn opening_resets_the_review() {
    let directory = temp_dir("reset");
    let good = png_file(&directory, "reset.png");
    let mut app = state(&directory);
    app.local_import_review.add_paths(&[good]);
    app.local_import_review.delete_after_success = true;

    let _ = open(&mut app);

    assert!(app.local_import_review.paths.is_empty());
    assert!(!app.local_import_review.delete_after_success);

    drop(app);
    let _ = std::fs::remove_dir_all(directory);
}
