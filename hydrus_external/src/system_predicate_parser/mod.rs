//! Rust port of `hydrus_python/hydrus/external/SystemPredicateParser.py`.
//!
//! The parser turns `system:` predicate text into a [`SystemPredicate`] with a
//! canonical predicate type, an optional operator, value and unit.

mod dates;
mod operators;
mod predicates;
mod ratings;
mod state;
mod table;
mod types;
mod units;
mod values;

use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException};

pub use predicates::{
    parse_system_predicate, parse_system_predicate_with_filetypes, string_looks_like_date,
};
pub use ratings::parse_service_specifier;
pub use types::{
    CanvasType, DateOrTimeInterval, FileServiceStatus, FiletypeRegistry, LogicalOperator,
    Operator, OperatorKind, Predicate, RatioOperator, RelationalOperator, ServiceSpecifier,
    SystemPredicate, TagDisplayType, Unit, UnitKind, ValueData, ValueKind,
};

pub(crate) fn parse_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::Parse, message)
}

#[cfg(test)]
mod tests;
