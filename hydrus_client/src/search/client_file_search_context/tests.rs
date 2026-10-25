use super::*;
use crate::search::client_number_test::{NUMBER_TEST_OPERATOR_EQUAL, NumberTest};

#[test]
fn empty_context_allows_everything() {
    let context = FileSearchContext::new();

    assert!(context.has_no_predicates());
    assert_eq!(context.get_summary(), "allows all files");
    assert!(!context.has_system_limit());
}

#[test]
fn tags_and_namespaces_are_split_by_inclusivity() {
    let mut excluded = Predicate::tag("bad tag");
    excluded.set_inclusive(false);

    let mut context = FileSearchContext::new();
    context.set_predicates(vec![
        Predicate::tag("blue eyes"),
        excluded,
        Predicate::namespace("creator"),
    ]);

    assert_eq!(context.get_tags_to_include(), vec!["blue eyes"]);
    assert_eq!(context.get_tags_to_exclude(), vec!["bad tag"]);
    assert_eq!(context.get_namespaces_to_include(), vec!["creator"]);
}

#[test]
fn serialisation_round_trips_nested_contexts() {
    let mut context = FileSearchContext::new();
    context.set_tag_service_key(b"local tags".to_vec());
    context.set_predicates(vec![Predicate::system_number(
        PredicateType::SystemSize,
        NumberTest::new(NUMBER_TEST_OPERATOR_EQUAL, 1024.0, None),
    )]);
    context.set_complete(true);

    let info = context.get_serialisable_info();
    let mut loaded = FileSearchContext::new();

    loaded.initialise_from_serialisable_info(&info).unwrap();

    assert_eq!(loaded, context);
    assert!(loaded.is_complete());
}

#[test]
fn media_result_test_checks_inbox_and_mime() {
    use hydrus_core::files::hydrus_file_handling::FileInfo;
    use hydrus_core::hydrus_constants::Mime;

    let mut result = MediaResult::new(
        vec![1],
        FileInfo {
            size: 1024,
            mime: Mime::Png,
            width: None,
            height: None,
            duration_ms: None,
            num_frames: None,
            has_audio: false,
            num_words: None,
        },
    );
    result.locations_manager.inbox = true;

    let mut context = FileSearchContext::new();
    context.set_predicates(vec![
        Predicate::new(PredicateType::SystemInbox, PredicateValue::None),
        Predicate::new(
            PredicateType::SystemMime,
            PredicateValue::Numbers(vec![Mime::Png.as_int() as i32]),
        ),
    ]);

    assert!(context.test_media_result(&result));
}

#[test]
fn summary_is_sorted_for_small_contexts() {
    let mut context = FileSearchContext::new();
    context.set_predicates(vec![Predicate::tag("zebra"), Predicate::tag("apple")]);

    assert_eq!(context.get_summary(), "apple, zebra");
}
