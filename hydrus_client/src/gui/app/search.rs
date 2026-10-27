//! Predicate model and system-predicate evaluation for query pages.
//!
//! The text in the search box is parsed once into tags and system predicates,
//! then the plan is executed against the client database. Tag predicates use
//! the tag-search indexes; file properties and timestamps are filtered in
//! memory over the candidate id set.

use std::collections::BTreeSet;

use hydrus_core::files::hydrus_file_handling::FileInfo;
use hydrus_core::hydrus_constants::{ContentStatus, TimestampType};
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;
use hydrus_core::hydrus_time;
use serde_json::{Value, json};

use crate::controller::Controller;
use crate::db::{
    files_storage, files_timestamps, files_viewing_stats, mappings, master_tags, notes, ratings,
    tag_search, url_map,
};
use crate::metadata::Rating;
use crate::search::client_file_system_predicates::FileSystemPredicates;
use crate::search::client_number_test::NumberTest;
use crate::search::client_search_predicate::{Predicate, PredicateValue};
use crate::search::parse_system_predicate;
use crate::search::predicate_constants::PredicateType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagTermKind {
    Tag,
    Namespace,
    Wildcard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagTerm {
    pub kind: TagTermKind,
    pub text: String,
    pub inclusive: bool,
}

#[derive(Debug, Clone)]
pub struct SearchPlan {
    pub tags: Vec<TagTerm>,
    pub predicates: Vec<Predicate>,
    pub system: FileSystemPredicates,
}

impl SearchPlan {
    pub fn limit(&self) -> Option<usize> {
        self.system
            .limit
            .and_then(|limit| usize::try_from(limit).ok())
    }

    /// A JSON description used by the Qt search sidebar.
    pub fn ui_json(&self) -> Value {
        let tags: Vec<Value> = self
            .tags
            .iter()
            .map(|term| {
                json!({
                    "kind": match term.kind {
                        TagTermKind::Tag => "tag",
                        TagTermKind::Namespace => "namespace",
                        TagTermKind::Wildcard => "wildcard",
                    },
                    "text": term.text,
                    "inclusive": term.inclusive,
                })
            })
            .collect();

        let predicates: Vec<Value> = self
            .predicates
            .iter()
            .map(|predicate| {
                json!({
                    "text": predicate.to_string(true),
                    "type": predicate.predicate_type.as_str(),
                })
            })
            .collect();

        json!({"tags": tags, "system": predicates})
    }
}

/// Parses the search box into the same shape Hydrus gets from autocomplete:
/// one predicate per entry, with multi-word tags merged when the database
/// already knows the phrase.
pub fn plan(db: &DBBase, query: &str) -> Result<SearchPlan, String> {
    let mut tags = Vec::new();
    let mut predicates = Vec::new();

    for term in split_terms(db, query) {
        let negative = term.starts_with('-') && term.len() > 1;
        let clean = if negative { &term[1..] } else { term.as_str() };

        if clean.is_empty() {
            continue;
        }

        if clean.to_ascii_lowercase().starts_with("system:") {
            let predicate = parse_system_predicate(clean)
                .map_err(|error| format!("could not parse '{clean}': {error}"))?;
            predicates.push(predicate);
            continue;
        }

        let (kind, text) = if clean.ends_with(":*") {
            (
                TagTermKind::Namespace,
                clean.trim_end_matches(":*").to_string(),
            )
        } else if clean.ends_with(':') {
            (
                TagTermKind::Namespace,
                clean.trim_end_matches(':').to_string(),
            )
        } else if clean.contains('*') {
            (TagTermKind::Wildcard, clean.to_string())
        } else {
            (TagTermKind::Tag, clean.to_string())
        };

        tags.push(TagTerm {
            kind,
            text,
            inclusive: !negative,
        });
    }

    let system = FileSystemPredicates::new(&predicates);

    Ok(SearchPlan {
        tags,
        predicates,
        system,
    })
}

/// Hydrus gets one predicate per autocomplete entry, so a tag such as
/// `blue eyes` is a single predicate even though it contains a space. The
/// plain line edit is parsed greedily: adjacent words are joined for as long
/// as the joined text is a known tag in the database.
fn split_terms(db: &DBBase, query: &str) -> Vec<String> {
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let mut terms = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        let token = tokens[index];

        if token.to_ascii_lowercase().starts_with("system:") {
            let mut phrase = token.to_string();
            let mut parsed = parse_system_predicate(&phrase).is_ok();
            index += 1;

            while index < tokens.len() {
                let next = tokens[index];

                if next.starts_with('-') || next.to_ascii_lowercase().starts_with("system:") {
                    break;
                }

                let candidate = format!("{phrase} {next}");

                if parse_system_predicate(&candidate).is_ok() {
                    phrase = candidate;
                    parsed = true;
                    index += 1;
                } else if parsed {
                    break;
                } else {
                    phrase = candidate;
                    index += 1;
                }
            }

            terms.push(phrase);
            continue;
        }

        let negative = token.starts_with('-') && token.len() > 1;
        let mut phrase = if negative {
            token[1..].to_string()
        } else {
            token.to_string()
        };
        index += 1;

        let mergeable = !phrase.ends_with(':') && !phrase.contains('*');

        if mergeable {
            while index < tokens.len() {
                let next = tokens[index];

                if next.starts_with('-') || next.to_ascii_lowercase().starts_with("system:") {
                    break;
                }

                let candidate = format!("{phrase} {next}");

                if master_tags::has_tag(db, &candidate).unwrap_or(false) {
                    phrase = candidate;
                    index += 1;
                } else {
                    break;
                }
            }
        }

        terms.push(if negative {
            format!("-{phrase}")
        } else {
            phrase
        });
    }

    terms
}

pub fn hash_ids_for_tag_term(
    db: &DBBase,
    tag_service_id: i64,
    term: &TagTerm,
) -> HydrusResult<BTreeSet<i64>> {
    let hash_ids = match term.kind {
        TagTermKind::Tag => tag_search::get_hash_ids_for_tag(
            db,
            tag_service_id,
            ContentStatus::Current,
            &term.text,
        )?,
        TagTermKind::Namespace => tag_search::get_hash_ids_for_namespace(
            db,
            tag_service_id,
            ContentStatus::Current,
            &term.text,
        )?,
        TagTermKind::Wildcard => tag_search::get_hash_ids_for_wildcard(
            db,
            tag_service_id,
            ContentStatus::Current,
            &term.text,
        )?,
    };

    Ok(hash_ids.into_iter().collect())
}

/// Returns `Some(matches)` for supported system predicates and `None` for the
/// ones this build cannot evaluate yet, so callers can ignore them instead of
/// dropping every result.
#[allow(clippy::too_many_arguments)]
pub fn matches_system(
    controller: &Controller,
    db: &DBBase,
    hash_id: i64,
    info: &FileInfo,
    tag_service_id: i64,
    file_service_id: Option<i64>,
    predicate: &Predicate,
) -> HydrusResult<Option<bool>> {
    let number = match &predicate.value {
        PredicateValue::Number(number) => Some(number),
        _ => None,
    };

    let result = match predicate.predicate_type {
        PredicateType::SystemSize => number.map(|test| test.test(Some(info.size as f64), None)),
        PredicateType::SystemWidth => {
            number.map(|test| test.test(info.width.map(|value| value as f64), None))
        }
        PredicateType::SystemHeight => {
            number.map(|test| test.test(info.height.map(|value| value as f64), None))
        }
        PredicateType::SystemRatio => number.map(|test| {
            let ratio = match (info.width, info.height) {
                (Some(width), Some(height)) if height > 0 => Some(width as f64 / height as f64),
                _ => None,
            };

            test.test(ratio, None)
        }),
        PredicateType::SystemDuration => {
            number.map(|test| test.test(info.duration_ms.map(|value| value as f64), None))
        }
        PredicateType::SystemFramerate => number.map(|test| {
            let framerate = match (info.num_frames, info.duration_ms) {
                (Some(frames), Some(duration)) if duration > 0 => {
                    Some(frames as f64 / (duration as f64 / 1000.0))
                }
                _ => None,
            };

            test.test(framerate, None)
        }),
        PredicateType::SystemNumFrames => {
            number.map(|test| test.test(info.num_frames.map(|value| value as f64), None))
        }
        PredicateType::SystemNumPixels => number.map(|test| {
            let pixels = match (info.width, info.height) {
                (Some(width), Some(height)) => Some(width as f64 * height as f64),
                _ => None,
            };

            test.test(pixels, None)
        }),
        PredicateType::SystemNumWords => {
            number.map(|test| test.test(info.num_words.map(|value| value as f64), None))
        }
        PredicateType::SystemHasAudio => Some(info.has_audio),
        PredicateType::SystemUntagged => Some(
            mappings::get_tag_ids(db, tag_service_id, ContentStatus::Current, hash_id)?.is_empty(),
        ),
        PredicateType::SystemNumTags => number.map(|test| {
            let namespace = predicate.get_namespace().unwrap_or("");
            let count = tag_count(db, tag_service_id, hash_id, namespace).unwrap_or(0);
            test.test(Some(count as f64), None)
        }),
        PredicateType::SystemNumUrls => number.map(|test| {
            let count = url_map::get_num_urls(db, hash_id).unwrap_or(0);
            test.test(Some(count as f64), None)
        }),
        PredicateType::SystemNumNotes => number.map(|test| {
            let count = notes::get_notes(db, hash_id)
                .map(|notes| notes.len())
                .unwrap_or(0);
            test.test(Some(count as f64), None)
        }),
        PredicateType::SystemImportTime
        | PredicateType::SystemArchivedTime
        | PredicateType::SystemModifiedTime
        | PredicateType::SystemLastViewedTime => {
            Some(date_matches(db, hash_id, file_service_id, predicate)?)
        }
        PredicateType::SystemRating => Some(rating_matches(controller, db, hash_id, predicate)?),
        _ => None,
    };

    Ok(result)
}

/// Applies the predicates the tag-store handles specially (mime and hash).
pub fn matches_mime_or_hash(
    hash: &[u8],
    info: &FileInfo,
    system: &FileSystemPredicates,
) -> HydrusResult<bool> {
    if !system.mimes.is_empty() && !system.mimes.contains(&info.mime) {
        return Ok(false);
    }

    if !system.hashes.is_empty() && !system.hashes.iter().any(|wanted| wanted == hash) {
        return Ok(false);
    }

    Ok(true)
}

pub fn tag_count(
    db: &DBBase,
    tag_service_id: i64,
    hash_id: i64,
    namespace: &str,
) -> HydrusResult<usize> {
    let tag_ids = mappings::get_tag_ids(db, tag_service_id, ContentStatus::Current, hash_id)?;
    let tags = master_tags::get_tags(db, &tag_ids)?;

    Ok(tags
        .iter()
        .filter(|tag| {
            namespace.is_empty()
                || tag
                    .split_once(':')
                    .is_some_and(|(tag_namespace, _)| tag_namespace == namespace)
        })
        .count())
}

fn date_matches(
    db: &DBBase,
    hash_id: i64,
    file_service_id: Option<i64>,
    predicate: &Predicate,
) -> HydrusResult<bool> {
    let PredicateValue::Date {
        operator,
        date_type,
        values,
    } = &predicate.value
    else {
        return Ok(true);
    };

    let target = if date_type == "date" && values.len() >= 5 {
        use chrono::NaiveDate;

        NaiveDate::from_ymd_opt(values[0], values[1] as u32, values[2] as u32)
            .and_then(|date| date.and_hms_opt(values[3] as u32, values[4] as u32, 0))
            .map(hydrus_time::datetime_to_timestamp)
            .unwrap_or(0)
    } else {
        let years = values.first().copied().unwrap_or(0) as i64;
        let months = values.get(1).copied().unwrap_or(0) as i64;
        let days = values.get(2).copied().unwrap_or(0) as i64;
        let hours = values.get(3).copied().unwrap_or(0) as i64;
        hydrus_time::get_now()
            - (years * 365 * 86_400 + months * 30 * 86_400 + days * 86_400 + hours * 3_600)
    };

    let timestamp = match predicate.predicate_type {
        PredicateType::SystemImportTime => file_service_id
            .map(|service_id| {
                files_storage::get_timestamp(db, service_id, ContentStatus::Current, hash_id)
            })
            .transpose()?
            .flatten(),
        PredicateType::SystemArchivedTime => {
            files_timestamps::get_file_timestamp(db, hash_id, TimestampType::Archived)?
        }
        PredicateType::SystemModifiedTime => {
            files_timestamps::get_file_timestamp(db, hash_id, TimestampType::ModifiedFile)?
        }
        PredicateType::SystemLastViewedTime => {
            files_viewing_stats::get_viewing_stats(db, hash_id, 0)?
                .last_viewed_timestamp_ms
                .map(|value| value / 1000)
        }
        _ => None,
    };

    let Some(timestamp) = timestamp else {
        return Ok(false);
    };

    Ok(compare(timestamp, target, operator))
}

fn rating_matches(
    controller: &Controller,
    db: &DBBase,
    hash_id: i64,
    predicate: &Predicate,
) -> HydrusResult<bool> {
    let PredicateValue::Service {
        operator: _,
        value: _,
        service_key,
    } = &predicate.value
    else {
        return Ok(true);
    };

    let manager = controller.services();
    let Some(service) = manager.get_service(service_key) else {
        return Ok(false);
    };

    let Some(rating) = ratings::get_rating(db, service.service_id, hash_id)? else {
        return Ok(false);
    };

    let PredicateValue::Service {
        operator,
        value,
        service_key: _,
    } = &predicate.value
    else {
        return Ok(true);
    };

    let test = NumberTest::new(*operator, value.unwrap_or(0.0), None);

    let rating_value = match rating {
        Rating::Numerical(value) => value,
        Rating::Like => 1.0,
        Rating::Dislike => 0.0,
    };

    Ok(test.test(Some(rating_value), None))
}

fn compare(left: i64, right: i64, operator: &str) -> bool {
    match operator {
        "<" => left < right,
        ">" => left > right,
        "\u{2264}" => left <= right,
        "\u{2265}" => left >= right,
        "\u{2260}" => left != right,
        _ => left == right,
    }
}

#[cfg(test)]
mod tests;
