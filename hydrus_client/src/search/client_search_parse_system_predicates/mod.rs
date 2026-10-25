//! Bridge from `hydrus/client/search/ClientSearchParseSystemPredicates.py` to
//! the already ported `hydrus_external::system_predicate_parser`.

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_external::system_predicate_parser::{self, SystemPredicate};

use crate::services::ServicesManager;

use crate::search::client_search_predicate::Predicate;

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

#[cfg(test)]
mod tests;
