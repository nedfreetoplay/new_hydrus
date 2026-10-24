use super::*;
use crate::system_predicate_parser::types::{
    DateOrTimeInterval, Operator, Predicate, RelationalOperator, Unit, ValueData,
};

fn parse(text: &str) -> SystemPredicate {
    parse_system_predicate(text).unwrap()
}

#[test]
fn simple_boolean_predicates_parse() {
    assert_eq!(parse("system:everything").predicate, Predicate::Everything);
    assert_eq!(parse("system:inbox").predicate, Predicate::Inbox);
    assert_eq!(parse("system:has tags").predicate, Predicate::HasTags);
    assert_eq!(parse("system:no notes").predicate, Predicate::NoNotes);
}

#[test]
fn numeric_predicates_parse_with_units() {
    let predicate = parse("system:width >= 500 pixels");

    assert_eq!(predicate.predicate, Predicate::Width);
    assert_eq!(
        predicate.operator,
        Some(Operator::Relational(RelationalOperator::GreaterEqual))
    );
    assert_eq!(predicate.value, Some(ValueData::Natural(500)));
    assert_eq!(predicate.unit, None);

    let predicate = parse("system:filesize > 2 MB");

    assert_eq!(predicate.predicate, Predicate::Filesize);
    assert_eq!(predicate.unit, Some(Unit::Megabytes));
}

#[test]
fn date_and_duration_predicates_parse() {
    let predicate = parse("system:modified before 2020-01-02");

    assert_eq!(predicate.predicate, Predicate::ModDate);
    assert!(matches!(
        predicate.value,
        Some(ValueData::DateOrTimeInterval(DateOrTimeInterval::DateTime(
            _
        )))
    ));

    let predicate = parse("system:duration > 1 hour 30 mins");

    assert_eq!(predicate.value, Some(ValueData::TimeToMsec(5_400_000)));
}

#[test]
fn hashes_and_ratings_parse() {
    let predicate = parse("system:hash is md5 abcdef0123456789");

    assert!(matches!(
        predicate.value,
        Some(ValueData::HashListWithAlgorithm { .. })
    ));

    let predicate = parse("system:rating my favourites > 3/5");

    assert_eq!(predicate.predicate, Predicate::RatingSpecificNumerical);
    assert!(matches!(
        predicate.value,
        Some(ValueData::RatingServiceNameAndNumericalValue { value: 3, .. })
    ));
}

#[test]
fn errors_are_reported() {
    for text in ["hello", "-system:inbox", "system:unknown", "system:width"] {
        assert!(parse_system_predicate(text).is_err(), "{text}");
    }
}

#[test]
fn filetypes_and_advanced_predicates_parse() {
    let predicate = parse("system:filetype = image/jpeg, png");

    assert_eq!(predicate.predicate, Predicate::Filetype);
    assert!(matches!(predicate.value, Some(ValueData::FiletypeList(_))));

    let predicate = parse("system:has tag in \"my tags\", ignoring siblings/parents: \"blah\"");

    assert_eq!(predicate.predicate, Predicate::TagAdvancedInclusive);
    assert!(matches!(
        predicate.value,
        Some(ValueData::TagAdvancedTag(ref tag)) if tag == "blah"
    ));

    let predicate = parse("system:all ratings rated");

    assert_eq!(predicate.predicate, Predicate::RatingAdvanced);
    assert!(matches!(
        predicate.value,
        Some(ValueData::RatingAdvanced {
            logical_operator: LogicalOperator::All,
            rated: true,
            ..
        })
    ));
}

#[test]
fn ratio_views_and_file_service_predicates_parse() {
    assert_eq!(
        parse("system:ratio wider than 16:9").value,
        Some(ValueData::Ratio {
            first: 16,
            second: 9
        })
    );

    assert!(matches!(
        parse("system:views in media, preview > 5").operator,
        Some(Operator::Views { .. })
    ));

    assert_eq!(
        parse("system:file service is currently in my files").operator,
        Some(Operator::FileService(FileServiceStatus::IsCurrentlyIn))
    );

    assert!(matches!(
        parse("system:tag as number page > 5").operator,
        Some(Operator::TagRelational { .. })
    ));
}
