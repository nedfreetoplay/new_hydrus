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
fn general_mime_classes_match_their_members() {
    let options = FileFilteringImportOptions {
        allowed_mimes: vec![Mime::GeneralImage],
        ..Default::default()
    };

    assert!(options.check_file_is_valid(1, Mime::Png, None, None));
    assert!(options.check_file_is_valid(1, Mime::Jpeg, None, None));
    assert!(!options.check_file_is_valid(1, Mime::Mp4Video, None, None));
    assert!(!options.check_file_is_valid(1, Mime::Pdf, None, None));
}

#[test]
fn animated_gif_size_limit_is_enforced() {
    let options = FileFilteringImportOptions {
        max_gif_size: Some(100),
        ..Default::default()
    };

    assert!(options.check_file_is_valid(100, Mime::AnimationGif, None, None));
    assert!(!options.check_file_is_valid(101, Mime::AnimationGif, None, None));
    assert!(options.check_file_is_valid(101, Mime::Png, None, None));
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
            .file_filtering_or_default()
            .check_file_is_valid(1, Mime::Png, None, None)
    );
    assert!(container.tags_or_default().is_empty());
    assert!(container.is_empty());
}

#[test]
fn container_serialises_option_types_and_skips_empty_ones() {
    let mut container = ImportOptionsContainer::default();
    container.file_filtering = Some(FileFilteringImportOptions {
        max_size: Some(1234),
        allowed_mimes: vec![Mime::GeneralImage],
        ..Default::default()
    });
    container.presentation = Some(PresentationImportOptions {
        presentation_status: PRESENTATION_STATUS_NEW_ONLY.to_string(),
        ..Default::default()
    });

    let value = serde_json::to_value(&container).unwrap();

    assert_eq!(value["file_filtering"]["max_size"], 1234);
    assert_eq!(value["file_filtering"]["allowed_mimes"][0], "image");
    assert_eq!(value["presentation"]["presentation_status"], "new");
    assert!(value["locations"].is_null());

    let loaded: ImportOptionsContainer = serde_json::from_value(value).unwrap();
    assert_eq!(loaded, container);
}

#[test]
fn tag_import_options_serialise_service_keys_as_hex() {
    let mut tags = TagImportOptions::new();
    tags.set_service_options(
        vec![0xab, 0xcd],
        ServiceTagImportOptions {
            get_tags: true,
            ..Default::default()
        },
    );
    let mut container = ImportOptionsContainer::default();
    container.tags = Some(tags);

    let value = serde_json::to_value(&container).unwrap();

    assert!(value["tags"]["abcd"]["get_tags"].as_bool().unwrap());

    let loaded: ImportOptionsContainer = serde_json::from_value(value).unwrap();
    assert!(
        loaded
            .tags
            .unwrap()
            .get_service_options(&[0xab, 0xcd])
            .is_some()
    );
}

#[test]
fn slices_fill_missing_and_overwrite_existing_entries() {
    let global = ImportOptionsContainer {
        file_filtering: Some(FileFilteringImportOptions {
            max_size: Some(10),
            ..Default::default()
        }),
        presentation: Some(PresentationImportOptions::default()),
        ..Default::default()
    };
    let local = ImportOptionsContainer {
        file_filtering: Some(FileFilteringImportOptions {
            max_size: Some(20),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut effective = ImportOptionsContainer::default();
    effective.fill_in_with_slice(&global, Some("global"));
    effective.fill_in_with_slice(&local, Some("local"));
    assert_eq!(
        effective.file_filtering.as_ref().unwrap().max_size,
        Some(10)
    );
    assert_eq!(effective.get_source_label("file_filtering"), "global");

    effective.overwrite_with_slice(&local, Some("local"));
    assert_eq!(
        effective.file_filtering.as_ref().unwrap().max_size,
        Some(20)
    );
    assert_eq!(effective.get_source_label("file_filtering"), "local");
}

#[test]
fn summaries_cover_each_option_type() {
    let mut container = ImportOptionsContainer::default();
    container.file_filtering = Some(FileFilteringImportOptions {
        max_size: Some(10),
        ..Default::default()
    });
    container.locations = Some(LocationImportOptions {
        destination_services: vec!["archive".to_string()],
        automatically_archives: true,
        ..Default::default()
    });

    assert!(
        container
            .get_summary("local_import")
            .contains("file filtering")
    );
    assert!(
        container
            .get_summary("local_import")
            .contains("imports to archive")
    );
    assert!(
        LocationImportOptions::default()
            .get_summary()
            .contains("default")
    );
}

#[test]
fn file_type_classes_have_extensions() {
    for class in file_type_classes() {
        assert!(
            !mime_class_extensions(class).is_empty(),
            "{} has no extensions",
            class.as_str()
        );
    }
}
