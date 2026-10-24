//! Relational operator parsing.

use fancy_regex::Regex;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::system_predicate_parser::parse_error;
use crate::system_predicate_parser::predicates::string_looks_like_date;
use crate::system_predicate_parser::types::{
    OperatorKind, RelationalOperator,
};

pub(crate) fn parse_operator_relational(
    string: &str,
    spec: OperatorKind,
) -> HydrusResult<(String, RelationalOperator)> {
    let exact = spec == OperatorKind::RelationalExact;

    if spec == OperatorKind::RelationalTime {
        let regex = Regex::new(r"\d.*").expect("valid date regex");

        if let Some(found) = regex
            .find(string)
            .map_err(|error| parse_error(error.to_string()))?
        {
            let op_string = &string[..found.start()];
            let string_result = &string[found.start()..];
            let looks_like_date = string_looks_like_date(string_result);
            let invert_ops = !looks_like_date;

            if op_string.contains("month") && looks_like_date {
                return Ok((string_result.to_string(), RelationalOperator::Approx));
            } else if op_string.contains("around") && !looks_like_date {
                return Ok((string_result.to_string(), RelationalOperator::Approx));
            } else if op_string.contains("day") && looks_like_date {
                return Ok((string_result.to_string(), RelationalOperator::Equal));
            } else if op_string.contains("since") {
                return Ok((
                    string_result.to_string(),
                    if invert_ops {
                        RelationalOperator::Less
                    } else {
                        RelationalOperator::Greater
                    },
                ));
            } else if op_string.contains("before") {
                return Ok((
                    string_result.to_string(),
                    if invert_ops {
                        RelationalOperator::Greater
                    } else {
                        RelationalOperator::Less
                    },
                ));
            }
        }
    }

    if let Some(rest) = string.strip_prefix("==") {
        return Ok((rest.to_string(), RelationalOperator::Equal));
    }

    if !exact {
        for (prefix, operator) in [
            ("!=", RelationalOperator::NotEqual),
            ("is not", RelationalOperator::NotEqual),
            ("isn't", RelationalOperator::NotEqual),
            ("~=", RelationalOperator::Approx),
        ] {
            if let Some(rest) = string.strip_prefix(prefix) {
                return Ok((rest.to_string(), operator));
            }
        }
    }

    let mut operators = vec![
        RelationalOperator::Equal,
        RelationalOperator::Less,
        RelationalOperator::Greater,
    ];

    if !exact {
        operators.push(RelationalOperator::NotEqual);
        operators.push(RelationalOperator::Approx);
    }

    for operator in operators {
        if let Some(rest) = string.strip_prefix(operator.as_str()) {
            return Ok((rest.to_string(), operator));
        }
    }

    if let Some(rest) = string.strip_prefix("is") {
        return Ok((rest.to_string(), RelationalOperator::Equal));
    }

    if string.starts_with("has") {
        return Ok((string.to_string(), RelationalOperator::Greater));
    }

    if string.starts_with("no") {
        return Ok((string.to_string(), RelationalOperator::Equal));
    }

    Err(parse_error("Invalid relational operator"))
}
