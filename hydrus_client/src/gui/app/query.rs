//! Tag search and media display helpers for query pages.

use std::collections::BTreeSet;
use std::path::PathBuf;

use hydrus_core::files::hydrus_file_handling::FileInfo;
use hydrus_core::hydrus_constants::{ContentStatus, ServiceType};

use crate::controller::Controller;
use crate::db::{files_metadata, files_storage, mappings, master, master_tags, tag_search};

use super::model::QueryPage;

pub const MAX_DISPLAYED_FILES: usize = 1000;

pub fn refresh_page(controller: &Controller, page: &mut QueryPage) {
    match run_query(controller, page) {
        Ok(()) => {}
        Err(error) => {
            page.hashes.clear();
            page.status = format!("search failed: {error}");
            page.empty_text = error;
        }
    }
}

fn run_query(controller: &Controller, page: &mut QueryPage) -> Result<(), String> {
    let db = controller.db().db();

    let service_ids = file_service_ids(controller, &page.location);
    let tag_service_id =
        tag_service_id(controller).ok_or_else(|| "no local tag service".to_string())?;

    let mut all: BTreeSet<i64> = BTreeSet::new();

    for service_id in service_ids {
        all.extend(
            files_storage::get_hash_ids(db, service_id, ContentStatus::Current)
                .map_err(|error| error.to_string())?,
        );
    }

    let inbox: BTreeSet<i64> = files_storage::get_inbox_hash_ids(db)
        .map_err(|error| error.to_string())?
        .into_iter()
        .collect();

    let mut result = all.clone();
    let mut limit: Option<usize> = None;

    for term in terms_for_query(db, page.query.as_str()) {
        let term = term.as_str();
        let negative = term.starts_with('-') && term.len() > 1;
        let clean = if negative { &term[1..] } else { term };

        if clean.is_empty() {
            continue;
        }

        let lower = clean.to_ascii_lowercase();

        let term_set: Option<BTreeSet<i64>> = if lower == "system:inbox" {
            Some(inbox.clone())
        } else if lower == "system:archive" {
            Some(all.difference(&inbox).copied().collect())
        } else if lower == "system:everything" || lower == "system:all" {
            Some(all.clone())
        } else if let Some(value) = lower.strip_prefix("system:limit=") {
            limit = value.parse::<usize>().ok();
            None
        } else if lower.starts_with("system:") {
            None
        } else if clean.ends_with(':') {
            Some(
                tag_search::get_hash_ids_for_namespace(
                    db,
                    tag_service_id,
                    ContentStatus::Current,
                    clean.trim_end_matches(':'),
                )
                .map_err(|error| error.to_string())?,
            )
        } else if clean.contains('*') {
            Some(
                tag_search::get_hash_ids_for_wildcard(
                    db,
                    tag_service_id,
                    ContentStatus::Current,
                    clean,
                )
                .map_err(|error| error.to_string())?,
            )
        } else {
            Some(
                tag_search::get_hash_ids_for_tag(db, tag_service_id, ContentStatus::Current, clean)
                    .map_err(|error| error.to_string())?,
            )
        };

        let Some(term_set) = term_set else {
            continue;
        };

        if negative {
            result.retain(|hash_id| !term_set.contains(hash_id));
        } else {
            result.retain(|hash_id| term_set.contains(hash_id));
        }

        if result.is_empty() {
            break;
        }
    }

    let total = result.len();

    if let Some(limit) = limit {
        result = result.into_iter().take(limit).collect();
    }

    let mut hashes = Vec::with_capacity(result.len());

    for hash_id in result {
        if let Some(hash) = master::get_hash(db, hash_id).map_err(|error| error.to_string())? {
            if files_metadata::get_file_info(db, hash_id)
                .map_err(|error| error.to_string())?
                .is_some()
            {
                hashes.push(hash);
            }
        }
    }

    page.hashes = hashes;

    let shown = page.hashes.len();

    page.empty_text = if total == 0 {
        if page.query.trim().is_empty() {
            "drag files here or use File > import files".to_string()
        } else {
            "no files match this search".to_string()
        }
    } else {
        String::new()
    };

    page.status = if total == 0 {
        "0 files".to_string()
    } else if shown < total || shown > MAX_DISPLAYED_FILES {
        format!("{total} files (showing {})", shown.min(MAX_DISPLAYED_FILES))
    } else if total == 1 {
        "1 file".to_string()
    } else {
        format!("{total} files")
    };

    Ok(())
}

/// Splits the search text into Hydrus-style predicates.
///
/// Hydrus gets one predicate per autocomplete entry, so a tag such as
/// `blue eyes` is a single predicate even though it contains a space. Here the
/// plain line edit is parsed greedily: adjacent words are joined for as long as
/// the joined text is a known tag in the database.
fn terms_for_query(db: &hydrus_core::hydrus_db_base::DBBase, query: &str) -> Vec<String> {
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let mut terms = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        let token = tokens[index];
        let negative = token.starts_with('-') && token.len() > 1;
        let mut phrase = if negative {
            token[1..].to_string()
        } else {
            token.to_string()
        };
        index += 1;

        let mergeable = !phrase.to_ascii_lowercase().starts_with("system:")
            && !phrase.ends_with(':')
            && !phrase.contains('*');

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

pub fn local_file_service_id(controller: &Controller, location: &str) -> Option<i64> {
    let manager = controller.services();
    let wanted = match location {
        "trash" => "trash",
        _ => "my files",
    };

    manager
        .get_service(wanted.as_bytes())
        .map(|service| service.service_id)
        .or_else(|| {
            manager
                .get_services(Some(&[ServiceType::LocalFileDomain]))
                .first()
                .map(|service| service.service_id)
        })
}

/// Local file domains covered by a page location.
pub fn file_service_ids(controller: &Controller, location: &str) -> Vec<i64> {
    let manager = controller.services();

    if location == "all local files" {
        let mut ids: Vec<i64> = manager
            .get_services(Some(&[ServiceType::LocalFileDomain]))
            .into_iter()
            .map(|service| service.service_id)
            .collect();

        if ids.is_empty() {
            if let Some(id) = local_file_service_id(controller, "my files") {
                ids.push(id);
            }
        }

        ids
    } else {
        local_file_service_id(controller, location)
            .into_iter()
            .collect()
    }
}

pub fn tag_service_id(controller: &Controller) -> Option<i64> {
    let manager = controller.services();

    manager
        .get_service(b"local tags")
        .map(|service| service.service_id)
        .or_else(|| {
            manager
                .get_services(Some(&[ServiceType::LocalTag]))
                .first()
                .map(|service| service.service_id)
        })
}

pub fn trash_service_id(controller: &Controller) -> Option<i64> {
    controller
        .services()
        .get_service(b"trash")
        .map(|service| service.service_id)
}

pub fn hash_id_for(controller: &Controller, hash: &[u8]) -> Option<i64> {
    master::find_hash_id(controller.db().db(), hash)
        .ok()
        .flatten()
}

pub fn file_info_for(controller: &Controller, hash: &[u8]) -> Option<FileInfo> {
    let hash_id = hash_id_for(controller, hash)?;
    files_metadata::get_file_info(controller.db().db(), hash_id)
        .ok()
        .flatten()
}

pub fn thumbnail_path(controller: &Controller, hash: &[u8]) -> Option<PathBuf> {
    let path = controller.files_manager().get_thumbnail_path(hash).ok()?;

    if path.exists() { Some(path) } else { None }
}

pub fn file_path(controller: &Controller, hash: &[u8]) -> Option<PathBuf> {
    let info = file_info_for(controller, hash)?;
    let path = controller
        .files_manager()
        .get_file_path(hash, info.mime, false)
        .ok()?;

    if path.exists() { Some(path) } else { None }
}

/// Path used by the preview canvas and viewer. Non-image formats have no Qt
/// renderer yet, so they show their generated thumbnail instead.
pub fn preview_path(controller: &Controller, hash: &[u8]) -> Option<PathBuf> {
    let info = file_info_for(controller, hash)?;

    if hydrus_core::hydrus_constants::IMAGES.contains(&info.mime) {
        file_path(controller, hash)
    } else {
        thumbnail_path(controller, hash)
    }
}

pub fn selection_tags(controller: &Controller, hash: &[u8]) -> Vec<String> {
    let Some(hash_id) = hash_id_for(controller, hash) else {
        return Vec::new();
    };

    let Some(tag_service_id) = tag_service_id(controller) else {
        return Vec::new();
    };

    let mut tags = mappings::get_tags(
        controller.db().db(),
        tag_service_id,
        ContentStatus::Current,
        hash_id,
    )
    .unwrap_or_default();

    tags.sort();

    tags
}

pub fn all_tags(controller: &Controller) -> Vec<String> {
    controller
        .db()
        .db()
        .query_all(
            "SELECT CASE WHEN n.namespace = '' THEN s.subtag
                         ELSE n.namespace || ':' || s.subtag END
             FROM tags t
             JOIN namespaces n ON t.namespace_id = n.namespace_id
             JOIN subtags s ON t.subtag_id = s.subtag_id
             ORDER BY 1",
            &[],
            |row| row.get(0),
        )
        .unwrap_or_default()
}

pub fn media_json(controller: &Controller, page: &QueryPage) -> Vec<serde_json::Value> {
    page.hashes
        .iter()
        .take(MAX_DISPLAYED_FILES)
        .map(|hash| {
            let thumb = thumbnail_path(controller, hash)
                .map(|path| path.display().to_string())
                .unwrap_or_default();

            serde_json::json!({
                "hash": hex(hash),
                "thumb": thumb,
            })
        })
        .collect()
}

pub fn viewer_position(page: &QueryPage, hash: &[u8]) -> usize {
    page.hashes
        .iter()
        .position(|candidate| candidate == hash)
        .unwrap_or(0)
}

pub fn neighbour(page: &QueryPage, hash: &[u8], direction: &str) -> Option<Vec<u8>> {
    if page.hashes.is_empty() {
        return None;
    }

    let index = page.hashes.iter().position(|candidate| candidate == hash)?;

    let next = if direction == "next" {
        (index + 1) % page.hashes.len()
    } else if index == 0 {
        page.hashes.len() - 1
    } else {
        index - 1
    };

    page.hashes.get(next).cloned()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn unhex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests;
