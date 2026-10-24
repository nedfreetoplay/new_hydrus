//! Unit parsing, ported from `parse_unit`.

use hydrus_core::hydrus_exceptions::HydrusResult;

use super::parse_error;
use super::types::{Unit, UnitKind};

pub(crate) fn parse_unit(
    string: &str,
    spec: Option<UnitKind>,
) -> HydrusResult<(String, Option<Unit>)> {
    let string = string.trim();

    let Some(spec) = spec else {
        return Ok((string.to_string(), None));
    };

    match spec {
        UnitKind::Filesize => {
            for (prefix, unit) in [
                ("b", Unit::Bytes),
                ("byte", Unit::Bytes),
                ("bytes", Unit::Bytes),
                ("kb", Unit::Kilobytes),
                ("kilobytes", Unit::Kilobytes),
                ("kilobyte", Unit::Kilobytes),
                ("mb", Unit::Megabytes),
                ("megabytes", Unit::Megabytes),
                ("megabyte", Unit::Megabytes),
                ("gb", Unit::Gigabytes),
                ("gigabytes", Unit::Gigabytes),
                ("gigabyte", Unit::Gigabytes),
            ] {
                if let Some(rest) = string.strip_prefix(prefix) {
                    return Ok((rest.to_string(), Some(unit)));
                }
            }

            Err(parse_error("Invalid unit, expected a filesize"))
        }
        UnitKind::FileRelationshipType => {
            for (prefix, unit) in [
                ("duplicates", Unit::Duplicates),
                ("alternates", Unit::Alternates),
                ("potential duplicates", Unit::PotentialDuplicates),
            ] {
                if let Some(rest) = string.strip_prefix(prefix) {
                    return Ok((rest.to_string(), Some(unit)));
                }
            }

            for prefix in [
                "not related/false positives",
                "not related/false positive",
                "not related",
                "false positives",
                "false positive",
            ] {
                if let Some(rest) = string.strip_prefix(prefix) {
                    return Ok((rest.to_string(), Some(Unit::NotRelatedOrFalsePositive)));
                }
            }

            Err(parse_error("Invalid unit, expected a file relationship"))
        }
        UnitKind::PixelsOrNone => {
            if string.is_empty() {
                return Ok((string.to_string(), None));
            }

            for prefix in ["pixels", "pixel", "px"] {
                if let Some(rest) = string.strip_prefix(prefix) {
                    return Ok((rest.to_string(), None));
                }
            }

            Err(parse_error("Invalid unit, expected no unit or pixels"))
        }
        UnitKind::Pixels => {
            for (prefix, unit) in [
                ("px", Unit::Pixels),
                ("pixels", Unit::Pixels),
                ("pixel", Unit::Pixels),
                ("kpx", Unit::Kilopixels),
                ("kilopixels", Unit::Kilopixels),
                ("kilopixel", Unit::Kilopixels),
                ("mpx", Unit::Megapixels),
                ("megapixels", Unit::Megapixels),
                ("megapixel", Unit::Megapixels),
            ] {
                if let Some(rest) = string.strip_prefix(prefix) {
                    return Ok((rest.to_string(), Some(unit)));
                }
            }

            Err(parse_error("Invalid unit, expected pixels"))
        }
        UnitKind::FpsOrNone => {
            if string.is_empty() {
                return Ok((string.to_string(), None));
            }

            if let Some(rest) = string.strip_prefix("fps") {
                return Ok((rest.to_string(), None));
            }

            Err(parse_error("Invalid unit, expected no unit or fps"))
        }
    }
}
