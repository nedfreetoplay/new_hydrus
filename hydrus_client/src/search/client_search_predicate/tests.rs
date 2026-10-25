use super::*;

use crate::search::predicate_constants::*;
use hydrus_core::hydrus_serialisable::SerialisableObject;

#[test]
fn tag_predicates_render_like_python() {
    let predicate = Predicate::tag("blue eyes");

    assert_eq!(predicate.to_string(true), "blue eyes");
    assert_eq!(predicate.get_namespace(), None);
    assert_eq!(
        predicate.get_texts_and_namespaces(),
        (Some("blue eyes".into()), None)
    );
}

#[test]
fn namespace_predicates_append_wildcard() {
    assert_eq!(Predicate::namespace("creator").to_string(true), "creator:*");
}

#[test]
fn negative_tag_predicates_get_a_prefix() {
    let mut predicate = Predicate::tag("blue eyes");
    predicate.set_inclusive(false);

    assert_eq!(predicate.to_string(true), "- blue eyes");
}

#[test]
fn number_predicates_render_operator() {
    let predicate = Predicate::system_number(
        PredicateType::SystemWidth,
        NumberTest::new(
            crate::search::client_number_test::NUMBER_TEST_OPERATOR_GREATER_THAN,
            1000.0,
            None,
        ),
    );

    assert_eq!(predicate.to_string(true), "width > 1000");
}

#[test]
fn serialisation_round_trips_tag_and_number() {
    for predicate in [
        Predicate::tag("blue eyes"),
        Predicate::system_number(PredicateType::SystemSize, NumberTest::new(2, 1024.0, None)),
    ] {
        let info = predicate.get_serialisable_info();
        let mut loaded = Predicate::tag("");

        loaded.initialise_from_serialisable_info(&info).unwrap();

        assert_eq!(loaded, predicate);
    }
}
