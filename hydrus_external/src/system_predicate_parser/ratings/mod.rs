//! Rating and service-specifier value parsing.

mod advanced;
mod services;

pub(crate) use advanced::parse_rating_advanced;
pub use services::parse_service_specifier;

use fancy_regex::Regex;
use hydrus_core::hydrus_exceptions::HydrusResult;

use super::operators::{operator_strings, parse_operator_relational};
use super::parse_error;
use super::state::ParseState;
use super::types::{OperatorKind, ValueData};

pub(crate) fn parse_rating_numerical(
    state: &mut ParseState,
    string: &str,
) -> HydrusResult<()> {
    let regex = Regex::new(r"^(?P<name>.+?)\s+(?P<num>\d+)/(?P<den>\d+)$")
        .expect("valid numerical rating regex");
    let captures = regex
        .captures(string)
        .map_err(|error| parse_error(error.to_string()))?
        .ok_or_else(|| parse_error("Invalid value, expected a numerical rating"))?;

    let service_name = captures["name"].to_string();
    let numerator: u32 = captures["num"].parse().unwrap_or(0);
    let denominator: u32 = captures["den"].parse().unwrap_or(0);

    if numerator > denominator {
        return Err(parse_error("Invalid value, rating value was out of bounds"));
    }

    state.text_remainder = String::new();
    state.value = Some(ValueData::RatingServiceNameAndNumericalValue {
        value: numerator,
        service_name,
    });

    Ok(())
}

pub(crate) fn parse_like_dislike(
    state: &mut ParseState,
    string: &str,
) -> HydrusResult<()> {
    let (value, mut service_name) = if let Some(name) = string.strip_suffix("dislike") {
        (0.0, name.to_string())
    } else if let Some(name) = string.strip_suffix("like") {
        (1.0, name.to_string())
    } else {
        return Err(parse_error("Invalid value, expected like/dislike"));
    };

    service_name = service_name.trim().to_string();

    for (operator_string, _) in operator_strings() {
        if let Some(name) = service_name.strip_suffix(operator_string) {
            service_name = name.trim().to_string();
            break;
        }
    }

    state.text_remainder = String::new();
    state.value = Some(ValueData::RatingServiceNameAndLikeDislike {
        value,
        service_name,
    });

    Ok(())
}

pub(crate) fn parse_rating_inc_dec(
    state: &mut ParseState,
    string: &str,
) -> HydrusResult<()> {
    let regex = Regex::new(r"^(?P<name>.+?)\s+(?P<num>\d+)$")
        .expect("valid inc/dec rating regex");
    let captures = regex
        .captures(string)
        .map_err(|error| parse_error(error.to_string()))?
        .ok_or_else(|| parse_error("Invalid value, expected an inc/dec rating"))?;

    state.text_remainder = String::new();
    state.value = Some(ValueData::RatingServiceNameAndIncDec {
        value: captures["num"].parse().unwrap_or(0),
        service_name: captures["name"].to_string(),
    });

    Ok(())
}

pub(crate) fn parse_namespace_and_num_tags(
    state: &mut ParseState,
    string: &str,
) -> HydrusResult<()> {
    let regex = Regex::new(r"^(?P<namespace>.+) tags (?P<operator>.+?)\s?(?P<num>\d+)\s*$")
        .expect("valid namespace tag regex");
    let captures = regex
        .captures(string)
        .map_err(|error| parse_error(error.to_string()))?
        .ok_or_else(|| parse_error("Invalid value, expected a namespace and tag count"))?;

    let mut namespace = captures["namespace"].to_string();

    if namespace == "unnamespaced" {
        namespace = String::new();
    }

    let (_, operator) =
        parse_operator_relational(&captures["operator"], OperatorKind::Relational)?;

    state.text_remainder = String::new();
    state.value = Some(ValueData::NamespaceAndNumTags {
        namespace,
        operator,
        num: captures["num"].parse().unwrap_or(0),
    });

    Ok(())
}
