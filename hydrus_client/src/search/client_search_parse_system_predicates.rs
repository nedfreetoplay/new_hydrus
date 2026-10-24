//! Bridge from `hydrus/client/search/ClientSearchParseSystemPredicates.py` to
//! the already ported `hydrus_external::system_predicate_parser`.

use hydrus_core::hydrus_exceptions::{HydrusException, HydrusResult};
use hydrus_external::system_predicate_parser::{
    self, Operator, RelationalOperator, SystemPredicate, Unit, ValueData,
};

use crate::search::client_number_test::{
    NumberTest, NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT, NUMBER_TEST_OPERATOR_EQUAL,
    NUMBER_TEST_OPERATOR_GREATER_THAN, NUMBER_TEST_OPERATOR_GREATER_THAN_OR_EQUAL_TO,
    NUMBER_TEST_OPERATOR_LESS_THAN, NUMBER_TEST_OPERATOR_LESS_THAN_OR_EQUAL_TO,
    NUMBER_TEST_OPERATOR_NOT_EQUAL,
};
use crate::search::client_search_predicate::{Predicate, PredicateValue};
use crate::search::predicate_constants::*;

/// `SystemPredicateParser.parse_system_predicate`, converted to a client predicate.
pub fn parse_system_predicate(text: &str) -> HydrusResult<Predicate> {
    let parsed = system_predicate_parser::parse_system_predicate(text)?;

    convert_system_predicate(parsed)
}

pub fn convert_system_predicate(parsed: SystemPredicate) -> HydrusResult<Predicate> {
    use hydrus_external::system_predicate_parser::Predicate as ExternalPredicate;

    let simple_type = match parsed.predicate {
        ExternalPredicate::Everything => Some(PREDICATE_TYPE_SYSTEM_EVERYTHING),
        ExternalPredicate::Inbox => Some(PREDICATE_TYPE_SYSTEM_INBOX),
        ExternalPredicate::Archive => Some(PREDICATE_TYPE_SYSTEM_ARCHIVE),
        ExternalPredicate::Untagged => Some(PREDICATE_TYPE_SYSTEM_UNTAGGED),
        ExternalPredicate::HasAudio => Some(PREDICATE_TYPE_SYSTEM_HAS_AUDIO),
        ExternalPredicate::HasTransparency => Some(PREDICATE_TYPE_SYSTEM_HAS_TRANSPARENCY),
        ExternalPredicate::HasExif => Some(PREDICATE_TYPE_SYSTEM_HAS_EXIF),
        ExternalPredicate::HasXmp => Some(PREDICATE_TYPE_SYSTEM_HAS_XMP),
        ExternalPredicate::HasIptc => Some(PREDICATE_TYPE_SYSTEM_HAS_IPTC),
        ExternalPredicate::HasIccProfile => Some(PREDICATE_TYPE_SYSTEM_HAS_ICC_PROFILE),
        ExternalPredicate::HasHumanReadableEmbeddedMetadata => {
            Some(PREDICATE_TYPE_SYSTEM_HAS_HUMAN_READABLE_EMBEDDED_METADATA)
        }
        ExternalPredicate::HasSoftwareSource => Some(PREDICATE_TYPE_SYSTEM_HAS_SOFTWARE_SOURCE),
        _ => None,
    };

    if let Some(predicate_type) = simple_type {
        return Ok(Predicate::new(predicate_type, PredicateValue::None));
    }

    let numeric_type = match parsed.predicate {
        ExternalPredicate::Width => Some(PREDICATE_TYPE_SYSTEM_WIDTH),
        ExternalPredicate::Height => Some(PREDICATE_TYPE_SYSTEM_HEIGHT),
        ExternalPredicate::Filesize => Some(PREDICATE_TYPE_SYSTEM_SIZE),
        ExternalPredicate::Duration => Some(PREDICATE_TYPE_SYSTEM_DURATION),
        ExternalPredicate::Framerate => Some(PREDICATE_TYPE_SYSTEM_FRAMERATE),
        ExternalPredicate::NumOfFrames => Some(PREDICATE_TYPE_SYSTEM_NUM_FRAMES),
        ExternalPredicate::NumOfTags | ExternalPredicate::NumOfTagsWithNamespace => {
            Some(PREDICATE_TYPE_SYSTEM_NUM_TAGS)
        }
        ExternalPredicate::NumOfWords => Some(PREDICATE_TYPE_SYSTEM_NUM_WORDS),
        ExternalPredicate::NumOfUrls => Some(PREDICATE_TYPE_SYSTEM_NUM_URLS),
        ExternalPredicate::NumNotes => Some(PREDICATE_TYPE_SYSTEM_NUM_NOTES),
        ExternalPredicate::NumPixels => Some(PREDICATE_TYPE_SYSTEM_NUM_PIXELS),
        ExternalPredicate::NumFileRels => Some(PREDICATE_TYPE_SYSTEM_FILE_RELATIONSHIPS_COUNT),
        ExternalPredicate::Ratio | ExternalPredicate::RatioSpecial => {
            Some(PREDICATE_TYPE_SYSTEM_RATIO)
        }
        ExternalPredicate::MediaViews
        | ExternalPredicate::PreviewViews
        | ExternalPredicate::AllViews
        | ExternalPredicate::NewViews
        | ExternalPredicate::MediaViewtime
        | ExternalPredicate::PreviewViewtime
        | ExternalPredicate::AllViewtime
        | ExternalPredicate::NewViewtime => Some(PREDICATE_TYPE_SYSTEM_FILE_VIEWING_STATS),
        ExternalPredicate::Limit => Some(PREDICATE_TYPE_SYSTEM_LIMIT),
        _ => None,
    };

    if let Some(predicate_type) = numeric_type {
        let number = make_number_test(&parsed)?;

        return Ok(Predicate::system_number(predicate_type, number));
    }

    if parsed.predicate == ExternalPredicate::Filetype {
        if let Some(ValueData::FiletypeList(mimes)) = &parsed.value {
            return Ok(Predicate::new(
                PREDICATE_TYPE_SYSTEM_MIME,
                PredicateValue::Numbers(mimes.iter().map(|mime| i32::from(*mime)).collect()),
            ));
        }
    }

    if parsed.predicate == ExternalPredicate::Hash {
        if let Some(ValueData::HashListWithAlgorithm { hashes, .. }) = &parsed.value {
            return Ok(Predicate::new(
                PREDICATE_TYPE_SYSTEM_HASH,
                PredicateValue::Hashes(
                    hashes
                        .iter()
                        .filter_map(|hash| hex_decode(hash))
                        .collect(),
                ),
            ));
        }
    }

    if matches!(
        parsed.predicate,
        ExternalPredicate::SimilarToFiles | ExternalPredicate::SimilarToData
    ) {
        let hashes = match &parsed.value {
            Some(ValueData::SimilarToHashListWithDistance {
                pixel_hashes,
                perceptual_hashes,
                ..
            }) => pixel_hashes
                .iter()
                .chain(perceptual_hashes.iter())
                .filter_map(|hash| hex_decode(hash))
                .collect(),
            _ => Vec::new(),
        };

        return Ok(Predicate::new(
            PREDICATE_TYPE_SYSTEM_SIMILAR_TO,
            PredicateValue::Hashes(hashes),
        ));
    }

    if matches!(
        parsed.predicate,
        ExternalPredicate::Url
            | ExternalPredicate::NoUrl
            | ExternalPredicate::Domain
            | ExternalPredicate::NoDomain
            | ExternalPredicate::UrlClass
            | ExternalPredicate::NoUrlClass
            | ExternalPredicate::UrlRegex
            | ExternalPredicate::NoUrlRegex
    ) {
        if let Some(ValueData::AnyString(text)) = &parsed.value {
            return Ok(Predicate::new(
                PREDICATE_TYPE_SYSTEM_KNOWN_URLS,
                PredicateValue::Text(text.clone()),
            ));
        }
    }

    Err(HydrusException::new(
        hydrus_core::hydrus_exceptions::ErrorKind::Parse,
        format!(
            "system predicate {:?} needs service/date conversion that is not ported yet",
            parsed.predicate
        ),
    ))
}

fn make_number_test(parsed: &SystemPredicate) -> HydrusResult<NumberTest> {
    let operator = match parsed.operator.as_ref() {
        Some(Operator::Relational(operator)) => relational_operator(*operator),
        Some(Operator::Ratio(operator)) => match operator {
            hydrus_external::system_predicate_parser::RatioOperator::Equal => NUMBER_TEST_OPERATOR_EQUAL,
            hydrus_external::system_predicate_parser::RatioOperator::WiderThan => {
                NUMBER_TEST_OPERATOR_GREATER_THAN
            }
            hydrus_external::system_predicate_parser::RatioOperator::TallerThan => {
                NUMBER_TEST_OPERATOR_LESS_THAN
            }
            hydrus_external::system_predicate_parser::RatioOperator::Approx => {
                NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT
            }
        },
        _ => NUMBER_TEST_OPERATOR_EQUAL,
    };

    let mut value = match parsed.value.as_ref() {
        Some(ValueData::Natural(value)) => *value as f64,
        Some(ValueData::Integer(value)) => *value as f64,
        Some(ValueData::Ratio { first, second }) => *first as f64 / (*second).max(1) as f64,
        Some(ValueData::TimeInterval {
            days,
            hours,
            minutes,
            seconds,
        }) => {
            (*days as f64 * 86_400.0)
                + (*hours as f64 * 3_600.0)
                + (*minutes as f64 * 60.0)
                + *seconds as f64
        }
        _ => return Err(parse_error("system predicate has no numeric value")),
    };

    if let Some(unit) = parsed.unit {
        value *= match unit {
            Unit::Kilobytes | Unit::Kilopixels => 1024.0,
            Unit::Megabytes | Unit::Megapixels => 1024.0 * 1024.0,
            Unit::Gigabytes => 1024.0 * 1024.0 * 1024.0,
            _ => 1.0,
        };
    }

    let extra_value = (operator == NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT).then_some(0.15);

    Ok(NumberTest::new(operator, value, extra_value))
}

fn relational_operator(operator: RelationalOperator) -> i32 {
    match operator {
        RelationalOperator::Equal => NUMBER_TEST_OPERATOR_EQUAL,
        RelationalOperator::NotEqual => NUMBER_TEST_OPERATOR_NOT_EQUAL,
        RelationalOperator::Less => NUMBER_TEST_OPERATOR_LESS_THAN,
        RelationalOperator::Greater => NUMBER_TEST_OPERATOR_GREATER_THAN,
        RelationalOperator::LessEqual => NUMBER_TEST_OPERATOR_LESS_THAN_OR_EQUAL_TO,
        RelationalOperator::GreaterEqual => NUMBER_TEST_OPERATOR_GREATER_THAN_OR_EQUAL_TO,
        RelationalOperator::Approx => NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT,
    }
}

fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

fn parse_error(message: &str) -> HydrusException {
    HydrusException::new(hydrus_core::hydrus_exceptions::ErrorKind::Parse, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_system_predicates() {
        let predicate = parse_system_predicate("system:inbox").unwrap();

        assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_INBOX);
    }

    #[test]
    fn parses_width_with_operator() {
        let predicate = parse_system_predicate("system:width > 1000").unwrap();
        let PredicateValue::Number(number) = predicate.value else {
            panic!("expected a number test");
        };

        assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_WIDTH);
        assert_eq!(number.operator, NUMBER_TEST_OPERATOR_GREATER_THAN);
        assert_eq!(number.value, 1000.0);
    }

    #[test]
    fn parses_filesize_units() {
        let predicate = parse_system_predicate("system:filesize > 1MB").unwrap();
        let PredicateValue::Number(number) = predicate.value else {
            panic!("expected a number test");
        };

        assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_SIZE);
        assert_eq!(number.value, 1024.0 * 1024.0);
    }

    #[test]
    fn parses_filetype_lists() {
        let predicate = parse_system_predicate("system:filetype is png").unwrap();
        let PredicateValue::Numbers(mimes) = predicate.value else {
            panic!("expected mime numbers");
        };

        assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_MIME);
        assert!(mimes.contains(&(hydrus_core::hydrus_constants::Mime::Png.as_int() as i32)));
    }

    #[test]
    fn parses_limit() {
        let predicate = parse_system_predicate("system:limit = 10").unwrap();
        let PredicateValue::Number(number) = predicate.value else {
            panic!("expected a number test");
        };

        assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_LIMIT);
        assert_eq!(number.value, 10.0);
    }

    #[test]
    fn unsupported_service_predicates_are_errors() {
        assert!(parse_system_predicate("system:rating > 3").is_err());
    }
}
