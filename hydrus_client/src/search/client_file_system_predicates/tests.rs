use super::*;
use crate::search::client_number_test::NUMBER_TEST_OPERATOR_EQUAL;
use crate::search::client_search_predicate::Predicate;

fn number(predicate_type: PredicateType, value: f64) -> Predicate {
    Predicate::system_number(
        predicate_type,
        NumberTest::new(NUMBER_TEST_OPERATOR_EQUAL, value, None),
    )
}

#[test]
fn simple_booleans_are_extracted() {
    let predicates = [
        Predicate::new(PredicateType::SystemInbox, PredicateValue::None),
        Predicate::new(PredicateType::SystemLocal, PredicateValue::None),
        Predicate::tag("blue eyes"),
        {
            let mut predicate = Predicate::tag("bad tag");
            predicate.set_inclusive(false);
            predicate
        },
    ];

    let system = FileSystemPredicates::new(&predicates);

    assert!(system.must_be_inbox());
    assert!(system.must_be_local());
    assert_eq!(system.tags_include, vec!["blue eyes"]);
    assert_eq!(system.tags_exclude, vec!["bad tag"]);
}

#[test]
fn mime_and_limit_are_extracted() {
    let predicates = [
        Predicate::new(
            PredicateType::SystemMime,
            PredicateValue::Numbers(vec![Mime::Png.as_int() as i32, Mime::Jpeg.as_int() as i32]),
        ),
        number(PredicateType::SystemLimit, 100.0),
    ];

    let system = FileSystemPredicates::new(&predicates);

    assert_eq!(system.get_allowed_filetypes(), &[Mime::Png, Mime::Jpeg]);
    assert_eq!(system.get_limit(true), Some(100));
}

#[test]
fn number_tags_are_grouped_by_namespace() {
    let predicate = number(PredicateType::SystemNumTags, 2.0);
    let system = FileSystemPredicates::new(&[predicate]);

    assert!(system.get_num_tags_number_tests().contains_key(""));
}

#[test]
fn size_predicates_are_kept_as_system() {
    let system = FileSystemPredicates::new(&[number(PredicateType::SystemSize, 1024.0)]);

    assert_eq!(system.other_system.len(), 1);
}
