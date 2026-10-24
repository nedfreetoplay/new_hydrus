//! Filetype list value parsing.

use std::collections::BTreeSet;

use fancy_regex::Regex;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::system_predicate_parser::parse_error;
use crate::system_predicate_parser::state::ParseState;
use crate::system_predicate_parser::types::{FiletypeRegistry, ValueData};

pub(super) fn parse_filetypes(
    state: &mut ParseState,
    string: &str,
    filetypes: &FiletypeRegistry,
) -> HydrusResult<()> {
    let mut names: Vec<&str> = filetypes.names().collect();

    if names.is_empty() {
        return Err(parse_error("Invalid value, expected a list of file types"));
    }

    names.sort_by_key(|name| std::cmp::Reverse(name.len()));

    let alternatives = names
        .iter()
        .map(|name| format!("(?:{})", fancy_regex::escape(name)))
        .collect::<Vec<_>>()
        .join("|");

    let regex = Regex::new(&format!(
        r"^(?:(?:{alternatives})(?:[\s,]+|$))+"
    ))
    .expect("valid filetype regex");

    let found = regex
        .find(string)
        .map_err(|error| parse_error(error.to_string()))?
        .ok_or_else(|| parse_error("Invalid value, expected a list of file types"))?;

    let mut values = BTreeSet::new();

    for part in found.as_str().replace(',', "|").split('|') {
        let name = part.trim();

        if let Some(filetype_values) = filetypes.get(name) {
            values.extend(filetype_values.iter().copied());
        }
    }

    state.value = Some(ValueData::FiletypeList(values));
    state.text_remainder = string[found.end()..].to_string();

    Ok(())
}
