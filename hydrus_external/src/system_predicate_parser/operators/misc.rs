//! Individual operator format parsers.

use hydrus_core::hydrus_exceptions::HydrusResult;

use super::operator_strings;
use crate::system_predicate_parser::parse_error;
use crate::system_predicate_parser::state::ParseState;
use crate::system_predicate_parser::types::{
    CanvasType, Operator, RatioOperator, RelationalOperator, ValueData,
};

pub(super) fn parse_views(state: &mut ParseState, string: &str) -> HydrusResult<()> {
    let mut string = string.to_string();
    let mut canvas_types = Vec::new();

    for (label, canvas_type) in [
        ("media", CanvasType::Media),
        ("preview", CanvasType::Preview),
        ("client api", CanvasType::ClientApi),
    ] {
        if string.contains(label) {
            canvas_types.push(canvas_type);
            string = string.replace(label, "");
        }
    }

    string = string.trim_start_matches([',', ' ']).to_string();

    let (remaining, relational) =
        super::parse_operator_relational(&string, crate::system_predicate_parser::types::OperatorKind::Relational)?;

    state.text_remainder = remaining;
    state.operator = Some(Operator::Views {
        canvas_types,
        relational,
    });

    Ok(())
}

pub(super) fn parse_number_test(state: &mut ParseState, string: &str) -> HydrusResult<()> {
    if string.starts_with("has") {
        state.operator = Some(Operator::Relational(RelationalOperator::Greater));
        return Ok(());
    }

    if string.starts_with("no") {
        state.operator = Some(Operator::Relational(RelationalOperator::Equal));
        return Ok(());
    }

    for (operator_string, operator) in operator_strings() {
        if let Some(rest) = string.strip_prefix(operator_string) {
            state.text_remainder = rest.to_string();
            state.operator = Some(Operator::Relational(*operator));
            return Ok(());
        }
    }

    Err(parse_error("Invalid relational operator"))
}

pub(super) fn parse_equal(state: &mut ParseState, string: &str) -> HydrusResult<()> {
    for (prefix, operator) in [
        ("==", RelationalOperator::Equal),
        ("\u{2260}", RelationalOperator::NotEqual),
        ("!=", RelationalOperator::NotEqual),
        ("=", RelationalOperator::Equal),
        ("is not", RelationalOperator::NotEqual),
        ("isn't", RelationalOperator::NotEqual),
        ("is", RelationalOperator::Equal),
    ] {
        if let Some(rest) = string.strip_prefix(prefix) {
            state.text_remainder = rest.to_string();
            state.operator = Some(Operator::Relational(operator));
            return Ok(());
        }
    }

    Err(parse_error("Invalid equality operator"))
}

pub(super) fn parse_only_equal(state: &mut ParseState, string: &str) -> HydrusResult<()> {
    for prefix in ["==", "=", "is"] {
        if let Some(rest) = string.strip_prefix(prefix) {
            state.text_remainder = rest.to_string();
            state.operator = Some(Operator::Relational(RelationalOperator::Equal));
            return Ok(());
        }
    }

    Err(parse_error("Invalid equality operator"))
}

pub(super) fn parse_ratio_operator(state: &mut ParseState, string: &str) -> HydrusResult<()> {
    for (prefix, operator) in [
        ("wider than", RatioOperator::WiderThan),
        ("taller than", RatioOperator::TallerThan),
        ("is wider than", RatioOperator::WiderThan),
        ("is taller than", RatioOperator::TallerThan),
        ("==", RatioOperator::Equal),
        ("=", RatioOperator::Equal),
        ("is", RatioOperator::Equal),
        ("~=", RatioOperator::Approx),
        ("\u{2248}", RatioOperator::Approx),
    ] {
        if let Some(rest) = string.strip_prefix(prefix) {
            state.text_remainder = rest.to_string();
            state.operator = Some(Operator::Ratio(operator));
            return Ok(());
        }
    }

    Err(parse_error("Invalid ratio operator"))
}

pub(super) fn parse_ratio_special(state: &mut ParseState, string: &str) -> HydrusResult<()> {
    for (prefix, operator) in [
        ("square", RatioOperator::Equal),
        ("portrait", RatioOperator::TallerThan),
        ("landscape", RatioOperator::WiderThan),
    ] {
        if string.contains(prefix) {
            state.text_remainder = String::new();
            state.operator = Some(Operator::Ratio(operator));
            state.value = Some(ValueData::Ratio { first: 1, second: 1 });
            return Ok(());
        }
    }

    Ok(())
}
