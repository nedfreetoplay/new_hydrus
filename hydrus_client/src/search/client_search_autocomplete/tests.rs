use super::*;

fn parsed(text: &str) -> ParsedAutocompleteText {
    ParsedAutocompleteText::new(text, TagAutocompleteOptions::default())
}

#[test]
fn whitespace_is_collapsed() {
    let text = parsed("  blue    eyes  ");

    assert_eq!(text.get_search_text(false, false), "blue eyes");
}

#[test]
fn namespace_and_wildcard_are_recognised() {
    assert!(parsed("creator:").is_namespace_search());
    assert!(parsed("*eyes*").is_explicit_wildcard(true));
    assert!(!parsed("blue eyes").is_explicit_wildcard(true));
}

#[test]
fn system_predicates_are_parsed() {
    let text = parsed("system:inbox");
    let predicates = text.get_valid_system_predicates().unwrap();

    assert_eq!(predicates.len(), 1);
    assert!(text.is_valid_system_predicate());
    assert!(text.is_possible_system_predicate());
}

#[test]
fn immediate_predicate_converts_tags_and_wildcards() {
    let tag = parsed("blue eyes")
        .get_immediate_file_search_predicate(true)
        .unwrap();

    assert_eq!(tag.predicate_type, PREDICATE_TYPE_TAG);

    let wildcard = parsed("*eyes*")
        .get_immediate_file_search_predicate(true)
        .unwrap();

    assert_eq!(wildcard.predicate_type, PREDICATE_TYPE_WILDCARD);
}

#[test]
fn add_tag_predicate_ignores_system_text() {
    assert!(parsed("system:inbox").get_add_tag_predicate().is_none());
    assert_eq!(
        parsed("blue eyes")
            .get_add_tag_predicate()
            .unwrap()
            .to_string(false),
        "blue eyes"
    );
}

#[test]
fn cache_filters_by_substring() {
    let cache = PredicateResultsCache::new(vec![
        Predicate::tag("blue eyes"),
        Predicate::tag("blonde hair"),
        Predicate::tag("green eyes"),
    ]);

    assert_eq!(cache.filter_predicates("eyes").len(), 2);
    assert!(cache.can_serve_tag_results(&parsed("blue"), false));
    assert!(!cache.can_serve_tag_results(&parsed("red"), false));
}
