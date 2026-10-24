//! Tag relational operator parsing.

use fancy_regex::Regex;
use hydrus_core::hydrus_exceptions::HydrusResult;

use super::operator_strings;
use crate::system_predicate_parser::parse_error;
use crate::system_predicate_parser::state::ParseState;
use crate::system_predicate_parser::types::{Operator, RelationalOperator};

pub(super) fn parse_tag_relational(state: &mut ParseState, string: &str) -> HydrusResult<()> {
    let alternatives = operator_strings()
        .iter()
        .map(|(text, _)| fancy_regex::escape(text))
        .collect::<Vec<_>>()
        .join("|");
    let regex = Regex::new(&format!(
        r"^(?P<namespace>.*)\s+(?P<op>({alternatives}))"
    ))
    .expect("valid tag relational regex");
    let captures = regex
        .captures(string)
        .map_err(|error| parse_error(error.to_string()))?
        .ok_or_else(|| {
            parse_error(
                "Invalid operator, expected a tag followed by a relational operator",
            )
        })?;

    let mut namespace = captures["namespace"].to_string();
    let operator_text = &captures["op"];

    if namespace == "any namespace" {
        namespace = "*".to_string();
    } else if namespace == "unnamespaced" {
        namespace = String::new();
    }

    let mut operator = operator_strings()
        .iter()
        .find(|(text, _)| text == operator_text)
        .map(|(_, operator)| *operator)
        .unwrap_or(RelationalOperator::Approx);

    if !matches!(
        operator,
        RelationalOperator::Less
            | RelationalOperator::Greater
            | RelationalOperator::Approx
    ) {
        operator = RelationalOperator::Approx;
    }

    state.text_remainder = string[captures.get(0).expect("whole match").end()..].to_string();
    state.operator = Some(Operator::TagRelational {
        namespace,
        relational: operator,
    });

    Ok(())
}
