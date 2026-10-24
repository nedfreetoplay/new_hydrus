//! Operator parsing, ported from `SystemPredicateParser.py`.

mod file_service;
mod misc;
mod rating;
mod relational;
mod tag_relational;
mod tag_advanced;

use std::sync::LazyLock;

use hydrus_core::hydrus_exceptions::HydrusResult;

use super::state::ParseState;
use super::types::{OperatorKind, RelationalOperator};

pub(crate) use relational::parse_operator_relational;

pub(crate) fn operator_strings() -> &'static Vec<(String, RelationalOperator)> {
    static OPERATORS: LazyLock<Vec<(String, RelationalOperator)>> = LazyLock::new(|| {
        let mut values = vec![
            ("=".to_string(), RelationalOperator::Equal),
            ("==".to_string(), RelationalOperator::Equal),
            ("is".to_string(), RelationalOperator::Equal),
            ("\u{2260}".to_string(), RelationalOperator::NotEqual),
            ("!=".to_string(), RelationalOperator::NotEqual),
            ("is not".to_string(), RelationalOperator::NotEqual),
            ("isn't".to_string(), RelationalOperator::NotEqual),
            ("<".to_string(), RelationalOperator::Less),
            ("fewer than".to_string(), RelationalOperator::Less),
            ("less than".to_string(), RelationalOperator::Less),
            (">".to_string(), RelationalOperator::Greater),
            ("more than".to_string(), RelationalOperator::Greater),
            ("greater than".to_string(), RelationalOperator::Greater),
            ("\u{2264}".to_string(), RelationalOperator::LessEqual),
            ("<=".to_string(), RelationalOperator::LessEqual),
            ("fewer than or equal to".to_string(), RelationalOperator::LessEqual),
            ("less than or equal to".to_string(), RelationalOperator::LessEqual),
            ("\u{2265}".to_string(), RelationalOperator::GreaterEqual),
            (">=".to_string(), RelationalOperator::GreaterEqual),
            ("more than or equal to".to_string(), RelationalOperator::GreaterEqual),
            ("greater than or equal to".to_string(), RelationalOperator::GreaterEqual),
            ("\u{2248}".to_string(), RelationalOperator::Approx),
            ("~=".to_string(), RelationalOperator::Approx),
            ("about".to_string(), RelationalOperator::Approx),
            ("is about".to_string(), RelationalOperator::Approx),
        ];

        values.sort_by_key(|(text, _)| std::cmp::Reverse(text.len()));

        values
    });

    &OPERATORS
}

pub(crate) fn parse_operator(
    state: &mut ParseState,
    spec: Option<OperatorKind>,
) -> HydrusResult<()> {
    let mut string = state.text_remainder.clone();

    while string.starts_with(':') || string.starts_with(' ') {
        string = string.trim().to_string();

        if let Some(rest) = string.strip_prefix(':') {
            string = rest.to_string();
        }
    }

    let Some(spec) = spec else {
        return Ok(());
    };

    match spec {
        OperatorKind::ViewsRelational => misc::parse_views(state, &string),
        OperatorKind::RelationalNumberTest => misc::parse_number_test(state, &string),
        OperatorKind::Relational
        | OperatorKind::RelationalExact
        | OperatorKind::RelationalTime => {
            let (remaining, operator) = parse_operator_relational(&string, spec)?;
            state.text_remainder = remaining;
            state.operator = Some(super::types::Operator::Relational(operator));
            Ok(())
        }
        OperatorKind::RelationalForRatingService => {
            rating::parse_rating_service_operator(state, &string)
        }
        OperatorKind::Equal => misc::parse_equal(state, &string),
        OperatorKind::FileServiceStatus => file_service::parse_file_service_status(state, &string),
        OperatorKind::TagRelational => tag_relational::parse_tag_relational(state, &string),
        OperatorKind::OnlyEqual => misc::parse_only_equal(state, &string),
        OperatorKind::RatioOperators => misc::parse_ratio_operator(state, &string),
        OperatorKind::RatioOperatorsSpecial => misc::parse_ratio_special(state, &string),
        OperatorKind::TagAdvancedGubbins => tag_advanced::parse_tag_advanced_operator(state, &string),
        OperatorKind::EqualNotConsuming => Ok(()),
    }
}
