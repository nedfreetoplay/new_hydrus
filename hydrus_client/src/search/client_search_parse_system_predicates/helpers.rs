//! Helpers for system predicate conversion.

use hydrus_core::hydrus_exceptions::{HydrusException, HydrusResult};
use hydrus_external::system_predicate_parser::{
    Operator, RelationalOperator, SystemPredicate, Unit, ValueData,
};

use crate::search::client_number_test::{
    NUMBER_TEST_OPERATOR_APPROXIMATE_PERCENT, NUMBER_TEST_OPERATOR_EQUAL,
    NUMBER_TEST_OPERATOR_GREATER_THAN, NUMBER_TEST_OPERATOR_GREATER_THAN_OR_EQUAL_TO,
    NUMBER_TEST_OPERATOR_LESS_THAN, NUMBER_TEST_OPERATOR_LESS_THAN_OR_EQUAL_TO,
    NUMBER_TEST_OPERATOR_NOT_EQUAL, NumberTest,
};
use crate::search::client_search_predicate::Predicate;
use crate::services::ServicesManager;

pub(super) fn make_number_test(parsed: &SystemPredicate) -> HydrusResult<NumberTest> {
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

pub(super) fn relational_operator(operator: RelationalOperator) -> i32 {
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

pub(super) fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

pub(super) fn resolve_service_key(
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

pub(super) fn parse_error(message: &str) -> HydrusException {
    HydrusException::new(hydrus_core::hydrus_exceptions::ErrorKind::Parse, message)
}
