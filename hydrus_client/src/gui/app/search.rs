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
    duplicates, files_storage, files_timestamps, files_viewing_stats, mappings, master,
    master_tags, notes, ratings, similar_files, tag_search, url_map,
};
use crate::metadata::Rating;
use crate::search::client_file_system_predicates::FileSystemPredicates;
use crate::search::client_number_test::NumberTest;
use crate::search::client_search_predicate::{Predicate, PredicateValue};
use crate::search::parse_system_predicate_with_services;
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
pub fn plan(
    db: &DBBase,
    services: &crate::services::ServicesManager,
    query: &str,
) -> Result<SearchPlan, String> {
    let mut tags = Vec::new();
    let mut predicates = Vec::new();

    for term in split_terms(db, services, query) {
        let negative = term.starts_with('-') && term.len() > 1;
        let clean = if negative { &term[1..] } else { term.as_str() };

        if clean.is_empty() {
            continue;
        }

        if clean.to_ascii_lowercase().starts_with("system:") {
            let predicate = parse_system_predicate_with_services(clean, services)
                .map_err(|error| format!("could not parse '{clean}': {error}"))?;
            predicates.push(predicate);
            continue;
        }

        if clean.starts_with('{') && clean.ends_with('}') {
            let members = parse_or_group(&clean[1..clean.len() - 1], services)?;
            let mut predicate =
                Predicate::new(PredicateType::OrContainer, PredicateValue::Or(members));
            predicate.inclusive = !negative;
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

/// One candidate in the search autocomplete dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutocompleteEntry {
    pub text: String,
    pub category: &'static str,
    pub count: Option<i64>,
}

/// Suggestions for the token currently being typed. Tags and namespaces come
/// from the tag indexes, so their counts match what a search would return.
pub fn autocomplete_entries(
    db: &DBBase,
    tag_service_id: i64,
    raw: &str,
    limit: usize,
) -> Vec<AutocompleteEntry> {
    let raw = raw.trim();
    let mut entries = Vec::new();

    if raw.to_ascii_lowercase().starts_with("system:") {
        for text in SYSTEM_PREDICATE_SUGGESTIONS {
            if text.contains(raw) || raw.eq_ignore_ascii_case("system:") {
                entries.push(AutocompleteEntry {
                    text: (*text).to_string(),
                    category: "system",
                    count: None,
                });
            }
        }

        return entries;
    }

    if let Some(namespace) = raw.strip_suffix(":*") {
        let prefix = format!("{namespace}:");

        if let Ok(tags) = crate::db::tag_search::suggest_tags(
            db,
            tag_service_id,
            ContentStatus::Current,
            &prefix,
            limit,
        ) {
            for tag in tags {
                entries.push(AutocompleteEntry {
                    text: tag.text,
                    category: "tags",
                    count: Some(tag.count),
                });
            }
        }

        entries.push(AutocompleteEntry {
            text: format!("{namespace}:*"),
            category: "wildcards",
            count: namespace_count(db, tag_service_id, namespace),
        });

        return entries;
    }

    if let Some(namespace) = raw.strip_suffix(':') {
        if let Ok(tags) = crate::db::tag_search::suggest_tags(
            db,
            tag_service_id,
            ContentStatus::Current,
            &format!("{namespace}:"),
            limit,
        ) {
            for tag in tags {
                entries.push(AutocompleteEntry {
                    text: tag.text,
                    category: "tags",
                    count: Some(tag.count),
                });
            }
        }

        return entries;
    }

    if raw.contains('*') {
        entries.push(AutocompleteEntry {
            text: raw.to_string(),
            category: "wildcards",
            count: crate::db::tag_search::get_hash_ids_for_wildcard(
                db,
                tag_service_id,
                ContentStatus::Current,
                raw,
            )
            .ok()
            .map(|ids| ids.len() as i64),
        });

        return entries;
    }

    if let Ok(tags) =
        crate::db::tag_search::suggest_tags(db, tag_service_id, ContentStatus::Current, raw, limit)
    {
        for tag in tags {
            entries.push(AutocompleteEntry {
                text: tag.text,
                category: "tags",
                count: Some(tag.count),
            });
        }
    }

    if let Ok(namespaces) = crate::db::tag_search::suggest_namespaces(
        db,
        tag_service_id,
        ContentStatus::Current,
        raw,
        limit,
    ) {
        for namespace in namespaces {
            entries.push(AutocompleteEntry {
                text: format!("{}:", namespace.text),
                category: "namespaces",
                count: Some(namespace.count),
            });
        }
    }

    entries
}

fn namespace_count(db: &DBBase, tag_service_id: i64, namespace: &str) -> Option<i64> {
    crate::db::tag_search::get_hash_ids_for_namespace(
        db,
        tag_service_id,
        ContentStatus::Current,
        namespace,
    )
    .ok()
    .map(|ids| ids.len() as i64)
}

const SYSTEM_PREDICATE_SUGGESTIONS: &[&str] = &[
    "system:everything",
    "system:inbox",
    "system:archive",
    "system:untagged",
    "system:has audio",
    "system:no audio",
    "system:has duration",
    "system:no duration",
    "system:has exif",
    "system:no exif",
    "system:has transparency",
    "system:no transparency",
    "system:has url",
    "system:no url",
    "system:has url matching regex ",
    "system:has url with domain ",
    "system:has url with class ",
    "system:best quality of group",
    "system:not the best quality of group",
    "system:similar to files ",
    "system:num file relationships > 0 duplicates",
    "system:num file relationships > 0 alternates",
    "system:num file relationships > 0 potential duplicates",
    "system:media views > 0",
    "system:preview views > 0",
    "system:all views > 0",
    "system:media viewtime > 0 seconds",
    "system:file service is currently in my files",
];

/// Hydrus gets one predicate per autocomplete entry, so a tag such as
/// `blue eyes` is a single predicate even though it contains a space. The
/// plain line edit is parsed greedily: adjacent words are joined for as long
/// as the joined text is a known tag in the database.
fn split_terms(
    db: &DBBase,
    services: &crate::services::ServicesManager,
    query: &str,
) -> Vec<String> {
    let tokens = tokenize_query(query);
    let mut terms = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        let token = tokens[index].as_str();

        if token.trim_start_matches('-').starts_with('{') {
            terms.push(token.to_string());
            index += 1;
            continue;
        }

        if token.to_ascii_lowercase().starts_with("system:") {
            let mut phrase = token.to_string();
            let mut parsed = parse_system_predicate_with_services(&phrase, services).is_ok();
            index += 1;

            while index < tokens.len() {
                let next = tokens[index].as_str();

                if next.starts_with('-')
                    || next.to_ascii_lowercase().starts_with("system:")
                    || next.starts_with('{')
                {
                    break;
                }

                let candidate = format!("{phrase} {next}");

                if parse_system_predicate_with_services(&candidate, services).is_ok() {
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
                let next = tokens[index].as_str();

                if next.starts_with('-')
                    || next.to_ascii_lowercase().starts_with("system:")
                    || next.starts_with('{')
                {
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

/// Splits on whitespace but keeps `{ a | b }` OR groups together, including
/// nested groups and spaces inside predicates.
fn tokenize_query(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;

    for character in query.chars() {
        if character == '{' {
            depth += 1;
        } else if character == '}' {
            depth = depth.saturating_sub(1);
        }

        if character.is_whitespace() && depth == 0 {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }

            continue;
        }

        current.push(character);
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// Parses the inside of `{ ... }`. Members are separated by top-level `|`;
/// each member is one predicate, and a nested `{ ... }` member becomes another
/// OR group.
fn parse_or_group(
    text: &str,
    services: &crate::services::ServicesManager,
) -> Result<Vec<Predicate>, String> {
    let mut members = Vec::new();

    for part in split_top_level(text, '|') {
        let part = part.trim();

        if part.is_empty() {
            continue;
        }

        members.push(parse_or_member(part, services)?);
    }

    if members.is_empty() {
        return Err("OR group is empty".to_string());
    }

    Ok(members)
}

fn parse_or_member(
    text: &str,
    services: &crate::services::ServicesManager,
) -> Result<Predicate, String> {
    let negative = text.starts_with('-') && text.len() > 1;
    let clean = if negative { text[1..].trim() } else { text };

    let mut predicate = if clean.starts_with('{') && clean.ends_with('}') {
        let members = parse_or_group(&clean[1..clean.len() - 1], services)?;
        Predicate::new(PredicateType::OrContainer, PredicateValue::Or(members))
    } else if clean.to_ascii_lowercase().starts_with("system:") {
        parse_system_predicate_with_services(clean, services)
            .map_err(|error| format!("could not parse '{clean}': {error}"))?
    } else if clean.ends_with(":*") {
        Predicate::namespace(clean.trim_end_matches(":*"))
    } else if clean.ends_with(':') {
        Predicate::namespace(clean.trim_end_matches(':'))
    } else if clean.contains('*') {
        Predicate::new(
            PredicateType::Wildcard,
            PredicateValue::Text(clean.to_string()),
        )
    } else {
        Predicate::tag(clean)
    };

    predicate.inclusive = !negative;

    Ok(predicate)
}

fn split_top_level(text: &str, separator: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;

    for character in text.chars() {
        if character == '{' {
            depth += 1;
        } else if character == '}' {
            depth = depth.saturating_sub(1);
        }

        if character == separator && depth == 0 {
            parts.push(std::mem::take(&mut current));
            continue;
        }

        current.push(character);
    }

    parts.push(current);

    parts
}

/// Evaluates an OR predicate (and any tag/system predicate inside it) against
/// one file. Nested groups recurse; unsupported system predicates never match.
#[allow(clippy::too_many_arguments)]
pub fn matches_predicate_hash(
    controller: &Controller,
    db: &DBBase,
    hash_id: i64,
    info: &FileInfo,
    tag_service_id: i64,
    file_service_id: Option<i64>,
    predicate: &Predicate,
) -> HydrusResult<bool> {
    if let PredicateValue::Or(members) = &predicate.value {
        let mut any = false;

        for member in members {
            if matches_predicate_hash(
                controller,
                db,
                hash_id,
                info,
                tag_service_id,
                file_service_id,
                member,
            )? {
                any = true;
                break;
            }
        }

        return Ok(predicate.inclusive == any);
    }

    let matched = match predicate.predicate_type {
        PredicateType::Tag | PredicateType::Namespace | PredicateType::Wildcard => {
            hash_has_tag_predicate(db, tag_service_id, hash_id, predicate)?
        }
        _ => matches_system(
            controller,
            db,
            hash_id,
            info,
            tag_service_id,
            file_service_id,
            predicate,
        )?
        .unwrap_or(false),
    };

    Ok(predicate.inclusive == matched)
}

fn hash_has_tag_predicate(
    db: &DBBase,
    tag_service_id: i64,
    hash_id: i64,
    predicate: &Predicate,
) -> HydrusResult<bool> {
    let PredicateValue::Text(text) = &predicate.value else {
        return Ok(false);
    };

    let tag_ids = mappings::get_tag_ids(db, tag_service_id, ContentStatus::Current, hash_id)?;
    let tags = master_tags::get_tags(db, &tag_ids)?;

    let matched = tags.iter().any(|tag| match predicate.predicate_type {
        PredicateType::Tag => tag.eq_ignore_ascii_case(text),
        PredicateType::Namespace => tag
            .split_once(':')
            .is_some_and(|(namespace, _)| namespace.eq_ignore_ascii_case(text)),
        PredicateType::Wildcard => wildcard_matches(text, tag),
        _ => false,
    });

    Ok(matched)
}

fn wildcard_matches(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let text: Vec<char> = text.to_lowercase().chars().collect();

    fn walk(pattern: &[char], text: &[char]) -> bool {
        match pattern.split_first() {
            None => text.is_empty(),
            Some(('*', rest)) => {
                if walk(rest, text) {
                    return true;
                }

                !text.is_empty() && walk(pattern, &text[1..])
            }
            Some((character, rest)) => match text.split_first() {
                Some((candidate, remaining)) if candidate == character => walk(rest, remaining),
                _ => false,
            },
        }
    }

    walk(&pattern, &text)
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
        PredicateType::SystemEverything => Some(true),
        PredicateType::SystemInbox => Some(files_storage::is_inbox(db, hash_id).unwrap_or(false)),
        PredicateType::SystemArchive => {
            Some(!files_storage::is_inbox(db, hash_id).unwrap_or(false))
        }
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
        PredicateType::SystemKnownUrls => {
            Some(known_url_matches(controller, db, hash_id, predicate)?)
        }
        PredicateType::SystemFileViewingStats => {
            Some(viewing_stats_matches(db, hash_id, predicate)?)
        }
        PredicateType::SystemFileService => {
            Some(file_service_matches(controller, db, hash_id, predicate)?)
        }
        PredicateType::SystemSimilarTo => Some(similar_matches(db, hash_id, predicate)?),
        PredicateType::SystemFileRelationshipsKing => {
            Some(duplicates::is_king(db, hash_id)? == predicate.inclusive)
        }
        PredicateType::SystemFileRelationshipsCount => {
            Some(relationships_matches(db, hash_id, predicate)?)
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

pub fn known_url_matches(
    controller: &Controller,
    db: &DBBase,
    hash_id: i64,
    predicate: &Predicate,
) -> HydrusResult<bool> {
    let PredicateValue::KnownUrl { rule_type, rule } = &predicate.value else {
        return Ok(false);
    };

    let urls = url_map::get_urls(db, hash_id)?;

    let matched = match rule_type.as_str() {
        "exact_match" => urls.iter().any(|url| url == rule),
        "regex" => regex::Regex::new(rule)
            .map(|regex| urls.iter().any(|url| regex.is_match(url)))
            .unwrap_or(false),
        "domain" => {
            let domain = rule.trim().trim_start_matches("www.").to_lowercase();

            urls.iter().any(|url| {
                let candidate = url
                    .split("://")
                    .nth(1)
                    .unwrap_or(url)
                    .split('/')
                    .next()
                    .unwrap_or("")
                    .to_lowercase();

                candidate == domain || candidate.ends_with(&format!(".{domain}"))
            })
        }
        "url_class" => controller
            .url_classes()
            .into_iter()
            .find(|class| class.name == *rule)
            .map(|class| urls.iter().any(|url| class.matches(url)))
            .unwrap_or(false),
        _ => false,
    };

    Ok(predicate.inclusive == matched)
}

fn viewing_stats_matches(db: &DBBase, hash_id: i64, predicate: &Predicate) -> HydrusResult<bool> {
    let PredicateValue::ViewingStats {
        canvas_types,
        operator,
        value,
        viewtime,
    } = &predicate.value
    else {
        return Ok(false);
    };

    let mut total_views = 0i64;
    let mut total_time = 0i64;

    for canvas in canvas_types {
        let canvas_type = match canvas.as_str() {
            "preview" => 1,
            "client api" => 2,
            _ => 0,
        };
        let stats = files_viewing_stats::get_viewing_stats(db, hash_id, canvas_type)?;

        total_views += stats.views;
        total_time += stats.viewtime_ms;
    }

    let actual = if *viewtime { total_time } else { total_views };
    let test = NumberTest::new(*operator, *value, None);

    Ok(predicate.inclusive == test.test(Some(actual as f64), None))
}

fn file_service_matches(
    controller: &Controller,
    db: &DBBase,
    hash_id: i64,
    predicate: &Predicate,
) -> HydrusResult<bool> {
    let PredicateValue::FileService {
        is_in,
        status,
        service_key,
    } = &predicate.value
    else {
        return Ok(false);
    };

    let manager = controller.services();
    let Some(service) = manager.get_service(service_key) else {
        return Ok(false);
    };

    let status = ContentStatus::try_from_int(*status as u8).unwrap_or(ContentStatus::Current);
    let hash_ids = files_storage::get_hash_ids(db, service.service_id, status)?;

    Ok(hash_ids.contains(&hash_id) == *is_in)
}

fn similar_matches(db: &DBBase, hash_id: i64, predicate: &Predicate) -> HydrusResult<bool> {
    let PredicateValue::Similar {
        hashes,
        distance,
        files,
    } = &predicate.value
    else {
        return Ok(false);
    };

    let mut matched = false;

    for hash in hashes {
        let mut target_ids = if *files {
            master::find_hash_id(db, hash)?.into_iter().collect()
        } else {
            similar_files::get_hash_ids_for_perceptual_hash(db, hash)?
        };

        if !*files {
            target_ids.extend(similar_files::get_hash_ids_for_pixel_hash(db, hash)?);
        }

        target_ids.sort_unstable();
        target_ids.dedup();

        for target_id in target_ids {
            let mut query_hashes = similar_files::get_perceptual_hashes(db, target_id)?;

            if let Some(pixel_hash) = similar_files::get_pixel_hash(db, target_id)? {
                query_hashes.push(pixel_hash);
            }

            for query in query_hashes {
                if similar_files::search_similar(db, &query, *distance, Some(target_id))?
                    .iter()
                    .any(|candidate| candidate.hash_id == hash_id)
                {
                    matched = true;
                    break;
                }

                if similar_files::get_hash_ids_for_pixel_hash(db, &query)?.contains(&hash_id) {
                    matched = true;
                    break;
                }
            }

            if matched {
                break;
            }
        }

        if matched {
            break;
        }
    }

    Ok(predicate.inclusive == matched)
}

fn relationships_matches(db: &DBBase, hash_id: i64, predicate: &Predicate) -> HydrusResult<bool> {
    let PredicateValue::Relationships { number, kind } = &predicate.value else {
        return Ok(false);
    };

    let actual = duplicates::get_relationship_count(db, hash_id, kind)?;
    let matched = number.test(Some(actual as f64), None);

    Ok(predicate.inclusive == matched)
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
