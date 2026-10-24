use super::*;

#[test]
fn size_and_mime_filters_work() {
    let options = FileFilteringImportOptions {
        min_size: Some(100),
        max_size: Some(1000),
        allowed_mimes: vec![Mime::Png, Mime::Jpeg],
        ..Default::default()
    };

    assert!(options.check_file_is_valid(500, Mime::Png, None, None));
    assert!(!options.check_file_is_valid(50, Mime::Png, None, None));
    assert!(!options.check_file_is_valid(5000, Mime::Png, None, None));
    assert!(!options.check_file_is_valid(500, Mime::StaticGif, None, None));
}

#[test]
fn resolution_requires_dimensions() {
    let options = FileFilteringImportOptions {
        min_resolution: Some((100, 100)),
        ..Default::default()
    };

    assert!(options.check_file_is_valid(1, Mime::Png, Some(100), Some(100)));
    assert!(!options.check_file_is_valid(1, Mime::Png, Some(99), Some(100)));
    assert!(!options.check_file_is_valid(1, Mime::Png, None, None));
}

#[test]
fn summary_describes_active_rules() {
    let options = FileFilteringImportOptions {
        max_size: Some(1000),
        allowed_mimes: vec![Mime::Png],
        ..Default::default()
    };

    let summary = options.get_summary();

    assert!(summary.contains("max size 1000"));
    assert!(summary.contains("png"));
}

#[test]
fn tag_options_are_keyed_by_service() {
    let mut options = TagImportOptions::new();
    options.set_service_options(
        b"local tags".to_vec(),
        ServiceTagImportOptions {
            get_tags: true,
            additional_tags: vec!["blue eyes".to_string()],
            ..Default::default()
        },
    );

    assert_eq!(
        options
            .get_service_options(b"local tags")
            .unwrap()
            .get_additional_tags(),
        ["blue eyes"]
    );
    assert_eq!(options.service_keys(), vec![b"local tags".as_slice()]);
}

#[test]
fn container_defaults_are_permissive() {
    let container = ImportOptionsContainer::default();

    assert!(
        container
            .file_filtering
            .check_file_is_valid(1, Mime::Png, None, None)
    );
    assert!(container.tag_import_options.is_empty());
}
