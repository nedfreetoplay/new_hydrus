//! Bridge from `hydrus/client/search/ClientSearchParseSystemPredicates.py` to
//! the already ported `hydrus_external::system_predicate_parser`.

use hydrus_core::hydrus_constants::ContentStatus;
use hydrus_core::hydrus_exceptions::{HydrusException, HydrusResult};
use hydrus_external::system_predicate_parser::{
    self, Operator, RelationalOperator, SystemPredicate, Unit, ValueData,
};

use crate::services::ServicesManager;

use crate::search::client_number_test::{
    NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT, NUMBER_TEST_OPERATOR_EQUAL,
    NUMBER_TEST_OPERATOR_GREATER_THAN, NUMBER_TEST_OPERATOR_GREATER_THAN_OR_EQUAL_TO,
    NUMBER_TEST_OPERATOR_LESS_THAN, NUMBER_TEST_OPERATOR_LESS_THAN_OR_EQUAL_TO,
    NUMBER_TEST_OPERATOR_NOT_EQUAL, NumberTest,
};
use crate::search::client_search_predicate::{Predicate, PredicateValue};
use crate::search::predicate_constants::*;

/// `SystemPredicateParser.parse_system_predicate`, converted to a client predicate.
pub fn parse_system_predicate(text: &str) -> HydrusResult<Predicate> {
    let parsed = system_predicate_parser::parse_system_predicate(text)?;

    convert(parsed, None)
}

/// Same as [`parse_system_predicate`], with access to service names for
/// file-service and rating predicates.
pub fn parse_system_predicate_with_services(
    text: &str,
    services: &ServicesManager,
) -> HydrusResult<Predicate> {
    let parsed = system_predicate_parser::parse_system_predicate(text)?;

    convert(parsed, Some(services))
}

pub fn convert_system_predicate(parsed: SystemPredicate) -> HydrusResult<Predicate> {
    convert(parsed, None)
}

fn convert(parsed: SystemPredicate, services: Option<&ServicesManager>) -> HydrusResult<Predicate> {
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
                PredicateValue::Hashes(hashes.iter().filter_map(|hash| hex_decode(hash)).collect()),
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

    if parsed.predicate == ExternalPredicate::FileService {
        let (is_in, status) = match parsed.operator {
            Some(Operator::FileService(status)) => match status {
                hydrus_external::system_predicate_parser::FileServiceStatus::IsCurrentlyIn => {
                    (true, ContentStatus::Current)
                }
                hydrus_external::system_predicate_parser::FileServiceStatus::IsNotCurrentlyIn => {
                    (false, ContentStatus::Current)
                }
                hydrus_external::system_predicate_parser::FileServiceStatus::IsPendingTo => {
                    (true, ContentStatus::Pending)
                }
                hydrus_external::system_predicate_parser::FileServiceStatus::IsNotPendingTo => {
                    (false, ContentStatus::Pending)
                }
            },
            _ => return Err(parse_error("file service predicate has no status")),
        };

        if let Some(ValueData::AnyString(service_name)) = &parsed.value {
            return Ok(Predicate::new(
                PREDICATE_TYPE_SYSTEM_FILE_SERVICE,
                PredicateValue::FileService {
                    is_in,
                    status: status.as_int() as i32,
                    service_key: resolve_service_key(services, service_name)?,
                },
            ));
        }
    }

    if matches!(
        parsed.predicate,
        ExternalPredicate::RatingSpecificNumerical
            | ExternalPredicate::RatingSpecificLikeDislike
            | ExternalPredicate::RatingSpecificIncDec
    ) {
        let operator = match parsed.operator.as_ref() {
            Some(Operator::Relational(operator)) => relational_operator(*operator),
            _ => NUMBER_TEST_OPERATOR_EQUAL,
        };

        let (value, service_name) = match &parsed.value {
            Some(ValueData::RatingServiceNameAndNumericalValue { value, service_name }) => {
                (Some(*value as f64), service_name.as_str())
            }
            Some(ValueData::RatingServiceNameAndLikeDislike { value, service_name }) => {
                (Some(*value), service_name.as_str())
            }
            Some(ValueData::RatingServiceNameAndIncDec { value, service_name }) => {
                (Some(*value as f64), service_name.as_str())
            }
            _ => return Err(parse_error("rating predicate has no value")),
        };

        return Ok(Predicate::new(
            PREDICATE_TYPE_SYSTEM_RATING,
            PredicateValue::Service {
                operator,
                value,
                service_key: resolve_service_key(services, service_name)?,
            },
        ));
    }

    let date_type = match parsed.predicate {
        ExternalPredicate::TimeImported => Some(PREDICATE_TYPE_SYSTEM_IMPORT_TIME),
        ExternalPredicate::ModDate => Some(PREDICATE_TYPE_SYSTEM_MODIFIED_TIME),
        ExternalPredicate::ArchivedDate => Some(PREDICATE_TYPE_SYSTEM_ARCHIVED_TIME),
        ExternalPredicate::LastViewedTime => Some(PREDICATE_TYPE_SYSTEM_LAST_VIEWED_TIME),
        _ => None,
    };

    if let Some(predicate_type) = date_type {
        let operator = match parsed.operator.as_ref() {
            Some(Operator::Relational(operator)) => operator.as_str().to_string(),
            _ => "=".to_string(),
        };

        let (date_type, values) = match parsed.value {
            Some(ValueData::DateOrTimeInterval(interval)) => match interval {
                hydrus_external::system_predicate_parser::DateOrTimeInterval::DateTime(datetime) => {
                    use chrono::{Datelike, Timelike};

                    (
                        "date".to_string(),
                        vec![
                            datetime.year(),
                            datetime.month() as i32,
                            datetime.day() as i32,
                            datetime.hour() as i32,
                            datetime.minute() as i32,
                        ],
                    )
                }
                hydrus_external::system_predicate_parser::DateOrTimeInterval::Interval {
                    years,
                    months,
                    days,
                    hours,
                } => (
                    "delta".to_string(),
                    vec![years as i32, months as i32, days as i32, hours as i32],
                ),
            },
            _ => return Err(parse_error("date predicate has no interval")),
        };

        return Ok(Predicate::new(
            predicate_type,
            PredicateValue::Date {
                operator,
                date_type,
                values,
            },
        ));
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
            hydrus_external::system_predicate_parser::RatioOperator::Equal => {
                NUMBER_TEST_OPERATOR_EQUAL
            }
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

fn resolve_service_key(
    services: Option<&ServicesManager>,
    service_name: &str,
) -> HydrusResult<Vec<u8>> {
    services
        .and_then(|services| services.get_service_key_from_name(service_name))
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            parse_error(&format!(
                "could not resolve service \"{service_name}\" (services manager required)"
            ))
        })
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

    fn services() -> (ServicesManager, std::path::PathBuf) {
        use crate::db::{ClientDB, services};
        use hydrus_core::hydrus_constants::ServiceType;

        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_predicate_services_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let db = ClientDB::new(&directory).unwrap();

        services::get_or_add_service(
            db.db(),
            b"my files",
            ServiceType::LocalFileDomain,
            "my files",
        )
        .unwrap();
        services::get_or_add_service(
            db.db(),
            b"my favourites",
            ServiceType::LocalRatingNumerical,
            "my favourites",
        )
        .unwrap();

        let manager = ServicesManager::load(db.db()).unwrap();

        (manager, directory)
    }

    #[test]
    fn file_service_predicates_resolve_service_names() {
        let (services, directory) = services();

        let predicate =
            parse_system_predicate_with_services("system:file service is currently in my files", &services)
                .unwrap();

        assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_FILE_SERVICE);
        assert_eq!(
            predicate.value,
            PredicateValue::FileService {
                is_in: true,
                status: hydrus_core::hydrus_constants::ContentStatus::Current.as_int() as i32,
                service_key: b"my files".to_vec(),
            }
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn rating_predicates_resolve_service_names() {
        let (services, directory) = services();

        let predicate =
            parse_system_predicate_with_services("system:rating my favourites > 3/5", &services)
                .unwrap();

        assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_RATING);

        let PredicateValue::Service {
            service_key,
            value,
            ..
        } = predicate.value
        else {
            panic!("expected a rating service predicate");
        };

        assert_eq!(service_key, b"my favourites".to_vec());
        assert!(value.is_some());

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn date_predicates_are_converted() {
        let predicate = parse_system_predicate("system:modified before 2020-01-02").unwrap();

        assert_eq!(predicate.predicate_type, PREDICATE_TYPE_SYSTEM_MODIFIED_TIME);

        let PredicateValue::Date {
            operator,
            date_type,
            values,
        } = predicate.value
        else {
            panic!("expected a date predicate");
        };

        assert_eq!(operator, "<");
        assert_eq!(date_type, "date");
        assert_eq!(&values[..3], &[2020, 1, 2]);
    }
}
