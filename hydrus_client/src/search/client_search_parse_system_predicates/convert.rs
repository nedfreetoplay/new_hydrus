//! System predicate conversion.

use hydrus_core::hydrus_constants::ContentStatus;
use hydrus_core::hydrus_exceptions::{HydrusException, HydrusResult};
use hydrus_external::system_predicate_parser::{Operator, SystemPredicate, ValueData};

use crate::search::client_number_test::NUMBER_TEST_OPERATOR_EQUAL;
use crate::search::client_search_predicate::{Predicate, PredicateValue};
use crate::search::predicate_constants::*;
use crate::services::ServicesManager;

use super::helpers::*;

pub(super) fn convert(
    parsed: SystemPredicate,
    services: Option<&ServicesManager>,
) -> HydrusResult<Predicate> {
    if let Some(predicate_type) = super::types::simple_type(&parsed) {
        return Ok(Predicate::new(predicate_type, PredicateValue::None));
    }

    if let Some(predicate_type) = super::types::numeric_type(&parsed) {
        let number = make_number_test(&parsed)?;

        return Ok(Predicate::system_number(predicate_type, number));
    }

    if let Some(predicate) = super::types::value_predicate(&parsed)? {
        return Ok(predicate);
    }

    convert_complex(parsed, services)
}

fn convert_complex(
    parsed: SystemPredicate,
    services: Option<&ServicesManager>,
) -> HydrusResult<Predicate> {
    use hydrus_external::system_predicate_parser::Predicate as ExternalPredicate;
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
                PredicateType::SystemFileService,
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
            Some(ValueData::RatingServiceNameAndNumericalValue {
                value,
                service_name,
            }) => (Some(*value as f64), service_name.as_str()),
            Some(ValueData::RatingServiceNameAndLikeDislike {
                value,
                service_name,
            }) => (Some(*value), service_name.as_str()),
            Some(ValueData::RatingServiceNameAndIncDec {
                value,
                service_name,
            }) => (Some(*value as f64), service_name.as_str()),
            _ => return Err(parse_error("rating predicate has no value")),
        };

        return Ok(Predicate::new(
            PredicateType::SystemRating,
            PredicateValue::Service {
                operator,
                value,
                service_key: resolve_service_key(services, service_name)?,
            },
        ));
    }

    let date_type = match parsed.predicate {
        ExternalPredicate::TimeImported => Some(PredicateType::SystemImportTime),
        ExternalPredicate::ModDate => Some(PredicateType::SystemModifiedTime),
        ExternalPredicate::ArchivedDate => Some(PredicateType::SystemArchivedTime),
        ExternalPredicate::LastViewedTime => Some(PredicateType::SystemLastViewedTime),
        _ => None,
    };

    if let Some(predicate_type) = date_type {
        let operator = match parsed.operator.as_ref() {
            Some(Operator::Relational(operator)) => operator.as_str().to_string(),
            _ => "=".to_string(),
        };

        let (date_type, values) = match parsed.value {
            Some(ValueData::DateOrTimeInterval(interval)) => match interval {
                hydrus_external::system_predicate_parser::DateOrTimeInterval::DateTime(
                    datetime,
                ) => {
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
