//! Simple, numeric and value predicate classification.

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_external::system_predicate_parser::{
    Predicate as ExternalPredicate, SystemPredicate, Unit, ValueData,
};

use crate::search::client_search_predicate::{Predicate, PredicateValue};
use crate::search::predicate_constants::*;

use super::helpers::*;

pub(super) fn simple_type(parsed: &SystemPredicate) -> Option<PredicateType> {
    let simple_type = match parsed.predicate {
        ExternalPredicate::Everything => Some(PredicateType::SystemEverything),
        ExternalPredicate::Inbox => Some(PredicateType::SystemInbox),
        ExternalPredicate::Archive => Some(PredicateType::SystemArchive),
        ExternalPredicate::Untagged => Some(PredicateType::SystemUntagged),
        ExternalPredicate::HasAudio => Some(PredicateType::SystemHasAudio),
        ExternalPredicate::HasTransparency => Some(PredicateType::SystemHasTransparency),
        ExternalPredicate::HasExif => Some(PredicateType::SystemHasExif),
        ExternalPredicate::HasXmp => Some(PredicateType::SystemHasXmp),
        ExternalPredicate::HasIptc => Some(PredicateType::SystemHasIptc),
        ExternalPredicate::HasIccProfile => Some(PredicateType::SystemHasIccProfile),
        ExternalPredicate::HasHumanReadableEmbeddedMetadata => {
            Some(PredicateType::SystemHasHumanReadableEmbeddedMetadata)
        }
        ExternalPredicate::HasSoftwareSource => Some(PredicateType::SystemHasSoftwareSource),
        _ => None,
    };

    simple_type
}

pub(super) fn numeric_type(parsed: &SystemPredicate) -> Option<PredicateType> {
    let numeric_type = match parsed.predicate {
        ExternalPredicate::Width => Some(PredicateType::SystemWidth),
        ExternalPredicate::Height => Some(PredicateType::SystemHeight),
        ExternalPredicate::Filesize => Some(PredicateType::SystemSize),
        ExternalPredicate::Duration => Some(PredicateType::SystemDuration),
        ExternalPredicate::Framerate => Some(PredicateType::SystemFramerate),
        ExternalPredicate::NumOfFrames => Some(PredicateType::SystemNumFrames),
        ExternalPredicate::NumOfTags | ExternalPredicate::NumOfTagsWithNamespace => {
            Some(PredicateType::SystemNumTags)
        }
        ExternalPredicate::NumOfWords => Some(PredicateType::SystemNumWords),
        ExternalPredicate::NumOfUrls => Some(PredicateType::SystemNumUrls),
        ExternalPredicate::NumNotes => Some(PredicateType::SystemNumNotes),
        ExternalPredicate::NumPixels => Some(PredicateType::SystemNumPixels),
        ExternalPredicate::Ratio | ExternalPredicate::RatioSpecial => {
            Some(PredicateType::SystemRatio)
        }
        ExternalPredicate::Limit => Some(PredicateType::SystemLimit),
        _ => None,
    };

    numeric_type
}

pub(super) fn value_predicate(parsed: &SystemPredicate) -> HydrusResult<Option<Predicate>> {
    if parsed.predicate == ExternalPredicate::Filetype {
        if let Some(ValueData::FiletypeList(mimes)) = &parsed.value {
            return Ok(Some(Predicate::new(
                PredicateType::SystemMime,
                PredicateValue::Numbers(mimes.iter().map(|mime| i32::from(*mime)).collect()),
            )));
        }
    }

    if parsed.predicate == ExternalPredicate::Hash {
        if let Some(ValueData::HashListWithAlgorithm { hashes, .. }) = &parsed.value {
            return Ok(Some(Predicate::new(
                PredicateType::SystemHash,
                PredicateValue::Hashes(hashes.iter().filter_map(|hash| hex_decode(hash)).collect()),
            )));
        }
    }

    if matches!(
        parsed.predicate,
        ExternalPredicate::BestQualityOfGroup | ExternalPredicate::NotBestQualityOfGroup
    ) {
        let mut predicate = Predicate::new(
            PredicateType::SystemFileRelationshipsKing,
            PredicateValue::None,
        );
        predicate.inclusive = parsed.predicate == ExternalPredicate::BestQualityOfGroup;

        return Ok(Some(predicate));
    }

    if matches!(
        parsed.predicate,
        ExternalPredicate::SimilarToFiles | ExternalPredicate::SimilarToData
    ) {
        let (hashes, distance, files) = match &parsed.value {
            Some(ValueData::SimilarToHashListWithDistance {
                pixel_hashes,
                perceptual_hashes,
                distance,
            }) => (
                pixel_hashes
                    .iter()
                    .chain(perceptual_hashes.iter())
                    .filter_map(|hash| hex_decode(hash))
                    .collect(),
                *distance as u32,
                false,
            ),
            Some(ValueData::HashListWithDistance { hashes, distance }) => (
                hashes.iter().filter_map(|hash| hex_decode(hash)).collect(),
                *distance as u32,
                true,
            ),
            _ => (
                Vec::new(),
                8,
                parsed.predicate == ExternalPredicate::SimilarToFiles,
            ),
        };

        return Ok(Some(Predicate::new(
            PredicateType::SystemSimilarTo,
            PredicateValue::Similar {
                hashes,
                distance,
                files,
            },
        )));
    }

    if parsed.predicate == ExternalPredicate::NumFileRels {
        let kind = match parsed.unit {
            Some(Unit::Alternates) => "alternates",
            Some(Unit::PotentialDuplicates) => "potential duplicates",
            Some(Unit::NotRelatedOrFalsePositive) => "not related/false positive",
            _ => "duplicates",
        };

        return Ok(Some(Predicate::new(
            PredicateType::SystemFileRelationshipsCount,
            PredicateValue::Relationships {
                number: make_number_test(parsed)?,
                kind: kind.to_string(),
            },
        )));
    }

    if matches!(
        parsed.predicate,
        ExternalPredicate::Url
            | ExternalPredicate::NoUrl
            | ExternalPredicate::Domain
            | ExternalPredicate::NoDomain
            | ExternalPredicate::UrlClass
            | ExternalPredicate::NoUrlClass
            | ExternalPredicate::UrlRegex
            | ExternalPredicate::NoUrlRegex
    ) {
        if let Some(ValueData::AnyString(text)) = &parsed.value {
            let (rule_type, inclusive) = match parsed.predicate {
                ExternalPredicate::Url => ("exact_match", true),
                ExternalPredicate::NoUrl => ("exact_match", false),
                ExternalPredicate::Domain => ("domain", true),
                ExternalPredicate::NoDomain => ("domain", false),
                ExternalPredicate::UrlClass => ("url_class", true),
                ExternalPredicate::NoUrlClass => ("url_class", false),
                ExternalPredicate::UrlRegex => ("regex", true),
                ExternalPredicate::NoUrlRegex => ("regex", false),
                _ => return Ok(None),
            };

            let mut predicate = Predicate::new(
                PredicateType::SystemKnownUrls,
                PredicateValue::KnownUrl {
                    rule_type: rule_type.to_string(),
                    rule: text.clone(),
                },
            );
            predicate.inclusive = inclusive;

            return Ok(Some(predicate));
        }
    }

    Ok(None)
}

/// Converts `media views`, `preview viewtime`, `views in X`, ... into one
/// viewing-stats predicate with summed canvas types.
pub(super) fn viewing_stats_predicate(parsed: &SystemPredicate) -> Option<Predicate> {
    use hydrus_external::system_predicate_parser::{CanvasType, Operator};

    let (default_canvas, viewtime) = match parsed.predicate {
        ExternalPredicate::MediaViews => ("media", false),
        ExternalPredicate::PreviewViews => ("preview", false),
        ExternalPredicate::AllViews | ExternalPredicate::NewViews => ("all", false),
        ExternalPredicate::MediaViewtime => ("media", true),
        ExternalPredicate::PreviewViewtime => ("preview", true),
        ExternalPredicate::AllViewtime | ExternalPredicate::NewViewtime => ("all", true),
        _ => return None,
    };

    let (canvas_types, relational) = match &parsed.operator {
        Some(Operator::Views {
            canvas_types,
            relational,
        }) => (
            canvas_types
                .iter()
                .map(|canvas| match canvas {
                    CanvasType::Media => "media".to_string(),
                    CanvasType::Preview => "preview".to_string(),
                    CanvasType::ClientApi => "client api".to_string(),
                })
                .collect(),
            Some(*relational),
        ),
        Some(Operator::Relational(relational)) => (canvas_names(default_canvas), Some(*relational)),
        _ => (canvas_names(default_canvas), None),
    };

    let value = match &parsed.value {
        Some(ValueData::Natural(value)) => *value as f64,
        Some(ValueData::Integer(value)) => *value as f64,
        Some(ValueData::TimeInterval {
            days,
            hours,
            minutes,
            seconds,
        }) => {
            ((*days as f64 * 86_400.0)
                + (*hours as f64 * 3_600.0)
                + (*minutes as f64 * 60.0)
                + *seconds as f64)
                * 1000.0
        }
        _ => return None,
    };

    let operator = relational
        .map(relational_operator)
        .unwrap_or(crate::search::client_number_test::NUMBER_TEST_OPERATOR_EQUAL);

    Some(Predicate::new(
        PredicateType::SystemFileViewingStats,
        PredicateValue::ViewingStats {
            canvas_types,
            operator,
            value,
            viewtime,
        },
    ))
}

fn canvas_names(canvas: &str) -> Vec<String> {
    match canvas {
        "all" => vec!["media".to_string(), "preview".to_string()],
        other => vec![other.to_string()],
    }
}
