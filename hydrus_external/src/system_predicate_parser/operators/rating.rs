//! Rating-service operator parsing.

use fancy_regex::Regex;
use hydrus_core::hydrus_exceptions::HydrusResult;

use super::operator_strings;
use crate::system_predicate_parser::parse_error;
use crate::system_predicate_parser::state::ParseState;
use crate::system_predicate_parser::types::{Operator, RelationalOperator};

pub(super) fn parse_rating_service_operator(
    state: &mut ParseState,
    string: &str,
) -> HydrusResult<()> {
    let regex = Regex::new(r"(?P<first>.*?)(?P<second>(dislike|like|\d+/\d+|\d+))$")
        .expect("valid rating operator regex");
    let captures = regex
        .captures(string)
        .map_err(|error| parse_error(error.to_string()))?
        .ok_or_else(|| parse_error("Invalid rating operator"))?;

    let without_value = captures["first"].trim();

    for (operator_string, operator) in operator_strings() {
        if let Some(service_name) = without_value.strip_suffix(operator_string) {
            if *operator == RelationalOperator::NotEqual {
                return Err(parse_error(
                    "Invalid rating operator--cannot select \"is not\"",
                ));
            }

            state.text_remainder = format!("{} {}", service_name.trim(), &captures["second"]);
            state.operator = Some(Operator::Relational(*operator));
            return Ok(());
        }
    }

    Err(parse_error("Invalid rating operator"))
}
