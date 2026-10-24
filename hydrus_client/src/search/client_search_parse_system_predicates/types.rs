//! Simple, numeric and value predicate classification.

use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_external::system_predicate_parser::{
    Predicate as ExternalPredicate, SystemPredicate, ValueData,
};

use crate::search::client_search_predicate::{Predicate, PredicateValue};
use crate::search::predicate_constants::*;

use super::helpers::*;

pub(super) fn simple_type(parsed: &SystemPredicate) -> Option<i32> {
    let simple_type = match parsed.predicate {
        ExternalPredicate::Everything => Some(PREDICATE_TYPE_SYSTEM_EVERYTHING),
        ExternalPredicate::Inbox => Some(PREDICATE_TYPE_SYSTEM_INBOX),
        ExternalPredicate::Archive => Some(PREDICATE_TYPE_SYSTEM_ARCHIVE),
        ExternalPredicate::Untagged => Some(PREDICATE_TYPE_SYSTEM_UNTAGGED),
        ExternalPredicate::HasAudio => Some(PREDICATE_TYPE_SYSTEM_HAS_AUDIO),
        ExternalPredicate::HasTransparency => Some(PREDICATE_TYPE_SYSTEM_HAS_TRANSPARENCY),
        ExternalPredicate::HasExif => Some(PREDICATE_TYPE_SYSTEM_HAS_EXIF),
        ExternalPredicate::HasXmp => Some(PREDICATE_TYPE_SYSTEM_HAS_XMP),
        ExternalPredicate::HasIptc => Some(PREDICATE_TYPE_SYSTEM_HAS_IPTC),
        ExternalPredicate::HasIccProfile => Some(PREDICATE_TYPE_SYSTEM_HAS_ICC_PROFILE),
        ExternalPredicate::HasHumanReadableEmbeddedMetadata => {
            Some(PREDICATE_TYPE_SYSTEM_HAS_HUMAN_READABLE_EMBEDDED_METADATA)
        }
        ExternalPredicate::HasSoftwareSource => Some(PREDICATE_TYPE_SYSTEM_HAS_SOFTWARE_SOURCE),
        _ => None,
    };

    simple_type
}

pub(super) fn numeric_type(parsed: &SystemPredicate) -> Option<i32> {
    let numeric_type = match parsed.predicate {
        ExternalPredicate::Width => Some(PREDICATE_TYPE_SYSTEM_WIDTH),
        ExternalPredicate::Height => Some(PREDICATE_TYPE_SYSTEM_HEIGHT),
        ExternalPredicate::Filesize => Some(PREDICATE_TYPE_SYSTEM_SIZE),
        ExternalPredicate::Duration => Some(PREDICATE_TYPE_SYSTEM_DURATION),
        ExternalPredicate::Framerate => Some(PREDICATE_TYPE_SYSTEM_FRAMERATE),
        ExternalPredicate::NumOfFrames => Some(PREDICATE_TYPE_SYSTEM_NUM_FRAMES),
        ExternalPredicate::NumOfTags | ExternalPredicate::NumOfTagsWithNamespace => {
            Some(PREDICATE_TYPE_SYSTEM_NUM_TAGS)
        }
        ExternalPredicate::NumOfWords => Some(PREDICATE_TYPE_SYSTEM_NUM_WORDS),
        ExternalPredicate::NumOfUrls => Some(PREDICATE_TYPE_SYSTEM_NUM_URLS),
        ExternalPredicate::NumNotes => Some(PREDICATE_TYPE_SYSTEM_NUM_NOTES),
        ExternalPredicate::NumPixels => Some(PREDICATE_TYPE_SYSTEM_NUM_PIXELS),
        ExternalPredicate::NumFileRels => Some(PREDICATE_TYPE_SYSTEM_FILE_RELATIONSHIPS_COUNT),
        ExternalPredicate::Ratio | ExternalPredicate::RatioSpecial => {
            Some(PREDICATE_TYPE_SYSTEM_RATIO)
        }
        ExternalPredicate::MediaViews
        | ExternalPredicate::PreviewViews
        | ExternalPredicate::AllViews
        | ExternalPredicate::NewViews
        | ExternalPredicate::MediaViewtime
        | ExternalPredicate::PreviewViewtime
        | ExternalPredicate::AllViewtime
        | ExternalPredicate::NewViewtime => Some(PREDICATE_TYPE_SYSTEM_FILE_VIEWING_STATS),
        ExternalPredicate::Limit => Some(PREDICATE_TYPE_SYSTEM_LIMIT),
        _ => None,
    };

    numeric_type
}

pub(super) fn value_predicate(parsed: &SystemPredicate) -> HydrusResult<Option<Predicate>> {
    if parsed.predicate == ExternalPredicate::Filetype {
        if let Some(ValueData::FiletypeList(mimes)) = &parsed.value {
            return Ok(Some(Predicate::new(
                PREDICATE_TYPE_SYSTEM_MIME,
                PredicateValue::Numbers(mimes.iter().map(|mime| i32::from(*mime)).collect()),
            )));
        }
    }

    if parsed.predicate == ExternalPredicate::Hash {
        if let Some(ValueData::HashListWithAlgorithm { hashes, .. }) = &parsed.value {
            return Ok(Some(Predicate::new(
                PREDICATE_TYPE_SYSTEM_HASH,
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
            PREDICATE_TYPE_SYSTEM_SIMILAR_TO,
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
                PREDICATE_TYPE_SYSTEM_KNOWN_URLS,
                PredicateValue::Text(text.clone()),
            )));
        }
    }

    Ok(None)
}
