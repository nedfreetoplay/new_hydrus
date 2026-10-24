//! The `SYSTEM_PREDICATES` table.

mod advanced;
mod basic;
mod numeric;
mod views;

use std::sync::LazyLock;

use fancy_regex::Regex;

use super::types::{OperatorKind, Predicate, UnitKind, ValueKind};

pub(crate) struct PredicateSpec {
    pub regex: Regex,
    pub predicate: Predicate,
    pub operator: Option<OperatorKind>,
    pub value: Option<ValueKind>,
    pub unit: Option<UnitKind>,
}

pub(super) fn spec(
    pattern: &str,
    predicate: Predicate,
    operator: Option<OperatorKind>,
    value: Option<ValueKind>,
    unit: Option<UnitKind>,
) -> PredicateSpec {
    let compiled = format!("^(?:{}):?", pattern.replace(' ', "([_ ]+)"));

    PredicateSpec {
        regex: Regex::new(&compiled).expect("valid system predicate regex"),
        predicate,
        operator,
        value,
        unit,
    }
}

pub(crate) fn predicate_specs() -> &'static Vec<PredicateSpec> {
    static SPECS: LazyLock<Vec<PredicateSpec>> = LazyLock::new(|| {
        let mut specs = Vec::new();

        specs.extend(basic::specs());
        specs.extend(numeric::specs());
        specs.extend(views::specs());
        specs.extend(advanced::specs());

        specs
    });

    &SPECS
}
