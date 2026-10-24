//! Parsing of the tag-advanced gubbins.

use fancy_regex::Regex;
use hydrus_core::hydrus_constants::ContentStatus;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::system_predicate_parser::parse_error;
use crate::system_predicate_parser::state::ParseState;
use crate::system_predicate_parser::types::{Operator, TagDisplayType};

fn split_not_in_quotes(text: &str, delimiter: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for character in text.chars() {
        if character == '"' {
            in_quotes = !in_quotes;
            current.push(character);
        } else if character == delimiter && !in_quotes {
            parts.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(character);
        }
    }

    parts.push(current.trim().to_string());

    parts
}

pub(crate) fn parse_tag_advanced_operator(
    state: &mut ParseState,
    string: &str,
) -> HydrusResult<()> {
    let colon_regex = Regex::new(r#"\:(?=(?:[^"]*"[^"]*")*[^"]*$)"#).expect("valid colon regex");
    let comma_regex = Regex::new(r#",(?=(?:[^"]*"[^"]*")*[^"]*$)"#).expect("valid comma regex");

    let colon_match = colon_regex
        .find(string)
        .map_err(|error| parse_error(error.to_string()))?;

    let (gubbins, tag) = if let Some(colon_match) = colon_match {
        (&string[..colon_match.start()], &string[colon_match.end()..])
    } else if comma_regex
        .find(string)
        .map_err(|error| parse_error(error.to_string()))?
        .is_some()
    {
        return Err(parse_error("Did not see a tag in the predicate string!"));
    } else {
        ("", string)
    };

    let mut service_name = None;
    let mut display_type = TagDisplayType::DisplayActual;
    let mut statuses = Vec::new();

    for component in split_not_in_quotes(gubbins, ',') {
        let quoted = Regex::new(r#"^[^"]*"(?P<name>.+)"[^"]*$"#).expect("valid quoted name regex");

        if let Some(captures) = quoted
            .captures(&component)
            .map_err(|error| parse_error(error.to_string()))?
        {
            service_name = Some(captures["name"].to_string());
            continue;
        }

        if component.contains("siblings") || component.contains("parents") {
            display_type = TagDisplayType::Storage;
            continue;
        }

        for (name, status) in [
            ("current", ContentStatus::Current),
            ("pending", ContentStatus::Pending),
            ("deleted", ContentStatus::Deleted),
            ("petitioned", ContentStatus::Petitioned),
        ] {
            if component.contains(name) {
                statuses.push(status);
            }
        }
    }

    if statuses.is_empty() {
        statuses = vec![ContentStatus::Current, ContentStatus::Pending];
    }

    state.text_remainder = tag.to_string();
    state.operator = Some(Operator::TagAdvanced {
        service_name,
        display_type,
        statuses,
    });

    Ok(())
}
