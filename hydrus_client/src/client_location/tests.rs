
use super::*;

fn key(value: &str) -> Vec<u8> {
    value.as_bytes().to_vec()
}

#[test]
fn simple_context_is_one_domain() {
    let context = LocationContext::static_create_simple(key("my files"));

    assert!(context.includes_current());
    assert!(!context.includes_deleted());
    assert!(context.is_one_domain());
    assert!(!context.is_all_known_files());
}

#[test]
fn deleted_inverse_swaps_sets() {
    let context = LocationContext::new([key("a")], [key("b")]);
    let inverse = context.get_deleted_inverse();

    assert_eq!(inverse.current_service_keys(), &BTreeSet::from([key("b")]));
    assert_eq!(inverse.deleted_service_keys(), &BTreeSet::from([key("a")]));
}

#[test]
fn statuses_match_python_order() {
    let context = LocationContext::new([key("a")], [key("b")]);
    let statuses = context.get_statuses_and_service_keys_list();

    assert_eq!(
        statuses,
        vec![
            (ContentStatus::Current, key("a")),
            (ContentStatus::Deleted, key("b"))
        ]
    );
}

#[test]
fn string_summary_uses_service_names() {
    let context = LocationContext::new([key("a")], [key("b")]);
    let text = context.to_string(|_| "test".to_string());

    assert_eq!(text, "a mix of current and deleted files of test, test");
}

#[test]
fn covering_keys_add_deleted_service() {
    let context = LocationContext::new([key("a")], [key("b")]);
    let (keys, cross_referenced) = context.get_covering_current_file_service_keys();

    assert!(keys.contains(&key("a")));
    assert!(keys.contains(&client_constants::COMBINED_DELETED_FILE_SERVICE_KEY.to_vec()));
    assert!(!cross_referenced);
}
