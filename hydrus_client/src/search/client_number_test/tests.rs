
use super::*;

#[test]
fn comparison_operators_match_python() {
    let less = NumberTest::new(NUMBER_TEST_OPERATOR_LESS_THAN, 10.0, None);

    assert!(less.test(Some(5.0), None));
    assert!(!less.test(Some(10.0), None));

    let greater = NumberTest::new(NUMBER_TEST_OPERATOR_GREATER_THAN, 10.0, None);

    assert!(greater.test(Some(11.0), None));
    assert!(!greater.test(None, None));
}

#[test]
fn none_handling_depends_on_threshold_sign() {
    let positive = NumberTest::new(NUMBER_TEST_OPERATOR_LESS_THAN, 5.0, None);
    let negative = NumberTest::new(NUMBER_TEST_OPERATOR_LESS_THAN, -5.0, None);

    assert!(positive.test(None, None));
    assert!(!negative.test(None, None));
}

#[test]
fn approximate_tests_use_extra_value() {
    let percent = NumberTest::new(NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT, 100.0, Some(0.15));

    assert!(percent.test(Some(90.0), None));
    assert!(!percent.test(Some(80.0), None));

    let absolute = NumberTest::new(NUMBER_TEST_OPERATOR_APPROXIMATE_ABSOLUTE, 100.0, Some(10.0));

    assert!(absolute.test(Some(105.0), None));
    assert!(!absolute.test(Some(111.0), None));
}

#[test]
fn zero_helpers_match_python() {
    assert!(NumberTest::new(NUMBER_TEST_OPERATOR_EQUAL, 0.0, None).is_zero());
    assert!(NumberTest::new(NUMBER_TEST_OPERATOR_LESS_THAN, 1.0, None).is_zero());
    assert!(NumberTest::new(NUMBER_TEST_OPERATOR_GREATER_THAN, 0.0, None).is_anything_but_zero());
}

#[test]
fn serialisation_round_trips_and_migrates_v1() {
    let test = NumberTest::new(NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT, 3.0, Some(0.15));
    let info = test.get_serialisable_info();
    let mut loaded = NumberTest::default();

    loaded.initialise_from_serialisable_info(&info).unwrap();
    assert_eq!(loaded, test);

    let (version, migrated) = test
        .update_serialisable_info(1, json!([NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT, 3.0]))
        .unwrap();

    assert_eq!(version, 2);
    assert_eq!(migrated[2], json!(0.15));
}
