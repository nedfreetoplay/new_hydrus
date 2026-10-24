//! Hash-list value parsing.

use std::collections::BTreeSet;

use fancy_regex::Regex;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::system_predicate_parser::parse_error;
use crate::system_predicate_parser::state::ParseState;
use crate::system_predicate_parser::types::ValueData;

pub(super) fn parse_hashes_with_distance(
    state: &mut ParseState,
    string: &str,
    similar: bool,
) -> HydrusResult<()> {
    let regex = Regex::new(
        r"^(?P<hashes>([0-9a-f]{4}[0-9a-f]+(\s|,)*)+)(with\s+)?(distance\s+)?(of\s+)?(?P<distance>0|([1-9][0-9]*))?",
    )
    .expect("valid hash distance regex");

    let captures = regex
        .captures(string)
        .map_err(|error| parse_error(error.to_string()))?
        .ok_or_else(|| {
            parse_error("Invalid value, expected a list of hashes with distance")
        })?;

    let hashes: BTreeSet<String> = captures["hashes"]
        .replace(',', " ")
        .split_whitespace()
        .filter(|hash| !hash.is_empty())
        .map(str::to_string)
        .collect();

    let distance = captures
        .name("distance")
        .and_then(|value| value.as_str().parse().ok())
        .unwrap_or(if similar { 8 } else { 4 });

    if similar {
        let pixel_hashes = hashes
            .iter()
            .filter(|hash| hash.len() == 64)
            .cloned()
            .collect();
        let perceptual_hashes = hashes
            .iter()
            .filter(|hash| hash.len() == 16)
            .cloned()
            .collect();

        state.value = Some(ValueData::SimilarToHashListWithDistance {
            pixel_hashes,
            perceptual_hashes,
            distance,
        });
    } else {
        state.value = Some(ValueData::HashListWithDistance { hashes, distance });
    }

    state.text_remainder = string[captures.get(0).expect("whole match").end()..].to_string();

    Ok(())
}

pub(super) fn parse_hashes_with_algorithm(
    state: &mut ParseState,
    string: &str,
) -> HydrusResult<()> {
    let mut algorithm = "sha256".to_string();

    for candidate in ["md5", "sha1", "sha512"] {
        if state.original_text_lower.contains(candidate) {
            algorithm = candidate.to_string();
            break;
        }
    }

    let regex = Regex::new(r"(?P<hashes>([0-9a-f]{8}[0-9a-f]+(\s|,)*)+)")
        .expect("valid hash algorithm regex");
    let found = regex
        .find(string)
        .map_err(|error| parse_error(error.to_string()))?
        .ok_or_else(|| {
            parse_error("Invalid value, expected a list of hashes and perhaps an algorithm")
        })?;

    let hashes: BTreeSet<String> = found
        .as_str()
        .replace(',', " ")
        .split_whitespace()
        .filter(|hash| !hash.is_empty())
        .map(str::to_string)
        .collect();

    state.value = Some(ValueData::HashListWithAlgorithm { hashes, algorithm });
    state.text_remainder = string[found.end()..].to_string();

    Ok(())
}
