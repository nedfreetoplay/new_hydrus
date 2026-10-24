//! Value parsing, ported from `parse_value`.

mod filetypes;
mod hashes;
mod time;

use fancy_regex::Regex;
use hydrus_core::hydrus_exceptions::HydrusResult;

use super::parse_error;
use super::state::ParseState;
use super::types::{FiletypeRegistry, ValueData, ValueKind};

pub(crate) fn parse_value(
    state: &mut ParseState,
    spec: Option<ValueKind>,
    filetypes: &FiletypeRegistry,
) -> HydrusResult<()> {
    let string = state.text_remainder.trim().to_string();

    let Some(spec) = spec else {
        return Ok(());
    };

    match spec {
        ValueKind::Natural | ValueKind::Integer => parse_integer(state, &string, spec),
        ValueKind::Sha256HashListWithDistance => {
            hashes::parse_hashes_with_distance(state, &string, false)
        }
        ValueKind::SimilarToHashListWithDistance => {
            hashes::parse_hashes_with_distance(state, &string, true)
        }
        ValueKind::HashListWithAlgorithm => hashes::parse_hashes_with_algorithm(state, &string),
        ValueKind::FiletypeList => filetypes::parse_filetypes(state, &string, filetypes),
        ValueKind::DateOrTimeInterval => super::dates::parse_date_or_time_interval(state, &string),
        ValueKind::TimeToMsec => time::parse_time_to_msec(state, &string),
        ValueKind::AnyString => parse_any_string(state, &string),
        ValueKind::TimeInterval => time::parse_time_interval(state, &string),
        ValueKind::Ratio => time::parse_ratio(state, &string),
        ValueKind::RatingServiceNameAndNumericalValue => {
            super::ratings::parse_rating_numerical(state, &string)
        }
        ValueKind::RatingServiceNameAndLikeDislike => {
            super::ratings::parse_like_dislike(state, &string)
        }
        ValueKind::RatingServiceNameAndIncDec => {
            super::ratings::parse_rating_inc_dec(state, &string)
        }
        ValueKind::NamespaceAndNumTags => {
            super::ratings::parse_namespace_and_num_tags(state, &string)
        }
        ValueKind::TagAdvancedTag => parse_tag_advanced_tag(state, &string),
        ValueKind::RatingAdvanced => super::ratings::parse_rating_advanced(state),
        ValueKind::RatioSpecial => Ok(()),
    }
}

fn parse_integer(state: &mut ParseState, string: &str, spec: ValueKind) -> HydrusResult<()> {
    if string.starts_with("has") || string.starts_with("no") {
        state.text_remainder = String::new();
        state.value = Some(match spec {
            ValueKind::Natural => ValueData::Natural(0),
            _ => ValueData::Integer(0),
        });

        return Ok(());
    }

    let regex = Regex::new(r"^-?[0-9,]+").expect("valid integer regex");

    let found = regex
        .find(string)
        .map_err(|error| parse_error(error.to_string()))?
        .ok_or_else(|| {
            if spec == ValueKind::Natural {
                parse_error("Invalid value, expected a natural number")
            } else {
                parse_error("Invalid value, expected an integer")
            }
        })?;

    let value: i64 = found
        .as_str()
        .replace(',', "")
        .parse()
        .map_err(|_| parse_error("Invalid value, expected an integer"))?;

    if spec == ValueKind::Natural {
        if value < 0 {
            return Err(parse_error("Invalid value, expected a positive integer!"));
        }

        state.value = Some(ValueData::Natural(value as u64));
    } else {
        state.value = Some(ValueData::Integer(value));
    }

    state.text_remainder = string[found.end()..].to_string();

    Ok(())
}

fn parse_any_string(state: &mut ParseState, string: &str) -> HydrusResult<()> {
    let value = if matches!(
        state.predicate,
        Some(super::types::Predicate::UrlRegex)
            | Some(super::types::Predicate::UrlClass)
            | Some(super::types::Predicate::NoUrlRegex)
    ) {
        state
            .original_text_lower
            .find(string)
            .map(|index| state.original_text[index..].to_string())
            .unwrap_or_else(|| string.to_string())
    } else {
        string.to_string()
    };

    state.text_remainder = String::new();
    state.value = Some(ValueData::AnyString(value));

    Ok(())
}

fn parse_tag_advanced_tag(state: &mut ParseState, string: &str) -> HydrusResult<()> {
    let quoted = Regex::new(r#"^[^"]*"(?P<tag>.+)"[^"]*$"#).expect("valid quoted tag regex");
    let raw_tag = quoted
        .captures(string)
        .map_err(|error| parse_error(error.to_string()))?
        .map(|captures| captures["tag"].to_string())
        .unwrap_or_else(|| string.to_string());

    let tag =
        hydrus_core::hydrus_tags::clean_tag(&raw_tag).unwrap_or_else(|_| "invalid tag".to_string());

    state.text_remainder = String::new();
    state.value = Some(ValueData::TagAdvancedTag(tag));

    Ok(())
}
