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

mod convert;
mod helpers;
mod types;

use convert::convert;
use helpers::*;

#[cfg(test)]
mod tests;
