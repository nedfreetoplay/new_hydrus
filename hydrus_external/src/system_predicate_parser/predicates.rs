//! Main `parse_system_predicate` entry point.

use hydrus_core::hydrus_exceptions::HydrusResult;

use super::operators::parse_operator;
use super::parse_error;
use super::state::ParseState;
use super::table::predicate_specs;
use super::types::{FiletypeRegistry, SystemPredicate};
use super::units::parse_unit;
use super::values::parse_value;

/// `string_looks_like_date( string )`
pub fn string_looks_like_date(string: &str) -> bool {
    !["year", "month", "day", "hour", "second", "ago"]
        .iter()
        .any(|word| string.contains(word))
}

/// `parse_system_predicate( string )` using Hydrus' default filetypes.
pub fn parse_system_predicate(input: &str) -> HydrusResult<SystemPredicate> {
    parse_system_predicate_with_filetypes(input, &FiletypeRegistry::hydrus_defaults())
}

/// `parse_system_predicate` with a caller-supplied filetype registry.
pub fn parse_system_predicate_with_filetypes(
    input: &str,
    filetypes: &FiletypeRegistry,
) -> HydrusResult<SystemPredicate> {
    let mut state = ParseState::new(input);

    check_basics(&state)?;

    for spec in predicate_specs() {
        let Some(found) = spec
            .regex
            .find(&state.subtag_text_lower)
            .map_err(|error| parse_error(error.to_string()))?
        else {
            continue;
        };

        state.text_remainder = state.subtag_text_lower[found.end()..].to_string();
        state.predicate = Some(spec.predicate);

        parse_operator(&mut state, spec.operator)?;
        parse_value(&mut state, spec.value, filetypes)?;

        let (remaining, unit) = parse_unit(&state.text_remainder, spec.unit)?;
        state.text_remainder = remaining;
        state.unit = unit;

        if !state.text_remainder.is_empty() {
            return Err(parse_error(format!(
                "Unrecognized characters at the end of the predicate: {input}"
            )));
        }

        return Ok(SystemPredicate {
            predicate: spec.predicate,
            operator: state.operator,
            value: state.value,
            unit: state.unit,
        });
    }

    Err(parse_error("Unknown system predicate!"))
}

fn check_basics(state: &ParseState) -> HydrusResult<()> {
    if state.original_text_lower.starts_with('-') {
        return Err(parse_error("System predicate can't start with negation"));
    }

    if !state.original_text_lower.starts_with("system:") {
        return Err(parse_error("Not a system predicate!"));
    }

    Ok(())
}
