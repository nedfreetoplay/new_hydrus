//! Simple, numeric and value predicate classification.

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_external::system_predicate_parser::{
    Predicate as ExternalPredicate, SystemPredicate, ValueData,
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
        ExternalPredicate::NumFileRels => Some(PredicateType::SystemFileRelationshipsCount),
        ExternalPredicate::Ratio | ExternalPredicate::RatioSpecial => {
            Some(PredicateType::SystemRatio)
        }
        ExternalPredicate::MediaViews
        | ExternalPredicate::PreviewViews
        | ExternalPredicate::AllViews
        | ExternalPredicate::NewViews
        | ExternalPredicate::MediaViewtime
        | ExternalPredicate::PreviewViewtime
        | ExternalPredicate::AllViewtime
        | ExternalPredicate::NewViewtime => Some(PredicateType::SystemFileViewingStats),
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
        ExternalPredicate::SimilarToFiles | ExternalPredicate::SimilarToData
    ) {
        let hashes = match &parsed.value {
            Some(ValueData::SimilarToHashListWithDistance {
                pixel_hashes,
                perceptual_hashes,
                ..
            }) => pixel_hashes
                .iter()
                .chain(perceptual_hashes.iter())
                .filter_map(|hash| hex_decode(hash))
                .collect(),
            _ => Vec::new(),
        };

        return Ok(Some(Predicate::new(
            PredicateType::SystemSimilarTo,
            PredicateValue::Hashes(hashes),
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
            return Ok(Some(Predicate::new(
                PredicateType::SystemKnownUrls,
                PredicateValue::Text(text.clone()),
            )));
        }
    }

    Ok(None)
}
