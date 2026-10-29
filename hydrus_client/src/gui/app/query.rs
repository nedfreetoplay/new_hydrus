//! Tag search and media display helpers for query pages.

use std::collections::BTreeSet;
use std::path::PathBuf;

use hydrus_core::files::hydrus_file_handling::FileInfo;
use hydrus_core::hydrus_constants::{ContentStatus, ServiceType};

use crate::controller::Controller;
use crate::db::{files_metadata, files_storage, mappings, master, url_map};

use super::model::QueryPage;
use super::search;

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

/// Runs a search box query and returns the matching hashes. Export folders use
/// this so their file selection is exactly the same as a search page.
pub fn hashes_for(
    controller: &Controller,
    query_text: &str,
    location: &str,
) -> Result<Vec<Vec<u8>>, String> {
    let mut page = QueryPage::new("export", "export", location);
    page.query = query_text.to_string();

    run_query(controller, &mut page)?;

    Ok(page.hashes)
}

fn run_query(controller: &Controller, page: &mut QueryPage) -> Result<(), String> {
    let db = controller.db().db();

    if let Some(fixed) = &page.fixed_hashes {
        page.hashes = fixed.clone();
        page.empty_text = if fixed.is_empty() {
            "no files were imported".to_string()
        } else {
            String::new()
        };
        page.status = match fixed.len() {
            0 => "0 files".to_string(),
            1 => "1 file".to_string(),
            count => format!("{count} files"),
        };

        return Ok(());
    }

    let service_ids = file_service_ids(controller, &page.location);
    let tag_service_id = tag_service_id_for(controller, &page.tag_service)
        .ok_or_else(|| "no local tag service".to_string())?;
    let plan = search::plan(db, &controller.services(), &page.query)?;

    let mut all: BTreeSet<i64> = BTreeSet::new();

    for service_id in &service_ids {
        all.extend(
            files_storage::get_hash_ids(db, *service_id, ContentStatus::Current)
                .map_err(|error| error.to_string())?,
        );
    }

    let inbox: BTreeSet<i64> = files_storage::get_inbox_hash_ids(db)
        .map_err(|error| error.to_string())?
        .into_iter()
        .collect();

    let mut result = all.clone();

    if plan.system.must_be_inbox() {
        result.retain(|hash_id| inbox.contains(hash_id));
    }

    if plan.system.must_be_archive() {
        result.retain(|hash_id| !inbox.contains(hash_id));
    }

    for term in &plan.tags {
        let term_set = search::hash_ids_for_tag_term(db, tag_service_id, term)
            .map_err(|error| error.to_string())?;

        if term.inclusive {
            result.retain(|hash_id| term_set.contains(hash_id));
        } else {
            result.retain(|hash_id| !term_set.contains(hash_id));
        }

        if result.is_empty() {
            break;
        }
    }

    for (namespace, tests) in &plan.system.num_tags {
        result.retain(|hash_id| {
            let count = search::tag_count(db, tag_service_id, *hash_id, namespace).unwrap_or(0);
            tests.iter().all(|test| test.test(Some(count as f64), None))
        });
    }

    if !plan.system.other_system.is_empty()
        || !plan.system.or_predicates.is_empty()
        || !plan.system.mimes.is_empty()
        || !plan.system.hashes.is_empty()
    {
        let file_service_id = service_ids.first().copied();
        let mut filtered = BTreeSet::new();

        for hash_id in result {
            let info = files_metadata::get_file_info(db, hash_id)
                .map_err(|error| error.to_string())?
                .unwrap_or(FileInfo {
                    size: 0,
                    mime: hydrus_core::hydrus_constants::Mime::Unknown,
                    width: None,
                    height: None,
                    duration_ms: None,
                    num_frames: None,
                    has_audio: false,
                    num_words: None,
                });

            let Some(hash) = master::get_hash(db, hash_id).map_err(|error| error.to_string())?
            else {
                continue;
            };

            if !search::matches_mime_or_hash(&hash, &info, &plan.system)
                .map_err(|error| error.to_string())?
            {
                continue;
            }

            let mut keep = true;

            for predicate in &plan.system.or_predicates {
                if !search::matches_predicate_hash(
                    controller,
                    db,
                    hash_id,
                    &info,
                    tag_service_id,
                    file_service_id,
                    predicate,
                )
                .map_err(|error| error.to_string())?
                {
                    keep = false;
                    break;
                }
            }

            if !keep {
                continue;
            }

            for predicate in &plan.system.other_system {
                match search::matches_system(
                    controller,
                    db,
                    hash_id,
                    &info,
                    tag_service_id,
                    file_service_id,
                    predicate,
                )
                .map_err(|error| error.to_string())?
                {
                    Some(false) => {
                        keep = false;
                        break;
                    }
                    _ => {}
                }
            }

            if keep {
                filtered.insert(hash_id);
            }
        }

        result = filtered;
    }

    let mut ordered: Vec<i64> = result.into_iter().collect();
    let total = ordered.len();

    if page.sort != "hash id" {
        let file_service_id = service_ids.first().copied();
        let mut keyed: Vec<(Option<f64>, i64)> = Vec::with_capacity(ordered.len());

        for hash_id in ordered {
            let info =
                files_metadata::get_file_info(db, hash_id).map_err(|error| error.to_string())?;

            let key = match page.sort.as_str() {
                "size" => info.as_ref().map(|info| info.size as f64),
                "width" => info
                    .as_ref()
                    .and_then(|info| info.width.map(|value| value as f64)),
                "height" => info
                    .as_ref()
                    .and_then(|info| info.height.map(|value| value as f64)),
                "ratio" => info
                    .as_ref()
                    .and_then(|info| match (info.width, info.height) {
                        (Some(width), Some(height)) if height > 0 => {
                            Some(width as f64 / height as f64)
                        }
                        _ => None,
                    }),
                "mime" => info
                    .as_ref()
                    .map(|info| i32::from(info.mime.as_int()) as f64),
                "duration" => info
                    .as_ref()
                    .and_then(|info| info.duration_ms.map(|value| value as f64)),
                "frames" => info
                    .as_ref()
                    .and_then(|info| info.num_frames.map(|value| value as f64)),
                "pixels" => info
                    .as_ref()
                    .and_then(|info| match (info.width, info.height) {
                        (Some(width), Some(height)) => Some(width as f64 * height as f64),
                        _ => None,
                    }),
                "words" => info
                    .as_ref()
                    .and_then(|info| info.num_words.map(|value| value as f64)),
                "import time" => file_service_id
                    .and_then(|service_id| {
                        files_storage::get_timestamp(
                            db,
                            service_id,
                            ContentStatus::Current,
                            hash_id,
                        )
                        .ok()
                        .flatten()
                    })
                    .map(|value| value as f64),
                _ => None,
            };

            keyed.push((key, hash_id));
        }

        keyed.sort_by(|left, right| {
            left.0
                .partial_cmp(&right.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ordered = keyed.into_iter().map(|(_, hash_id)| hash_id).collect();
    }

    if !page.sort_ascending {
        ordered.reverse();
    }

    if let Some(limit) = plan.limit() {
        ordered.truncate(limit);
    }

    let mut hashes = Vec::with_capacity(ordered.len());

    for hash_id in ordered {
        if let Some(hash) = master::get_hash(db, hash_id).map_err(|error| error.to_string())? {
            hashes.push(hash);
        }
    }

    page.hashes = hashes;
    page.selected.clear();

    let shown = page.hashes.len();

    page.empty_text = if total == 0 {
        if page.query.trim().is_empty() {
            "no search done yet".to_string()
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

/// Resolves the tag service selected on a query page, falling back to the
/// first local tag service when the page has no explicit selection.
pub fn tag_service_id_for(controller: &Controller, name: &str) -> Option<i64> {
    if name.is_empty() {
        return tag_service_id(controller);
    }

    let manager = controller.services();

    manager
        .get_service(name.as_bytes())
        .filter(|service| service.service_type == ServiceType::LocalTag)
        .map(|service| service.service_id)
        .or_else(|| tag_service_id(controller))
}

/// Names of the local tag services shown in the query page selector.
pub fn local_tag_service_names(controller: &Controller) -> Vec<String> {
    controller
        .services()
        .get_services(Some(&[ServiceType::LocalTag]))
        .into_iter()
        .map(|service| service.name.clone())
        .collect()
}

pub fn trash_service_id(controller: &Controller) -> Option<i64> {
    let manager = controller.services();

    manager
        .get_service(b"trash")
        .map(|service| service.service_id)
        .or_else(|| {
            manager
                .get_services(Some(&[ServiceType::LocalFileTrashDomain]))
                .first()
                .map(|service| service.service_id)
        })
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

pub fn urls_for(controller: &Controller, hash: &[u8]) -> Vec<String> {
    let Some(hash_id) = hash_id_for(controller, hash) else {
        return Vec::new();
    };

    url_map::get_urls(controller.db().db(), hash_id).unwrap_or_default()
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
    let db = controller.db().db();
    let mut duplicate_ids = BTreeSet::new();

    if let Ok(pairs) = crate::db::duplicates::get_potential_pairs(db, None) {
        for (smaller, larger, _) in pairs {
            duplicate_ids.insert(smaller);
            duplicate_ids.insert(larger);
        }
    }

    let rating_service = controller
        .services()
        .get_services(Some(&[
            ServiceType::LocalRatingNumerical,
            ServiceType::LocalRatingIncDec,
            ServiceType::LocalRatingLike,
        ]))
        .first()
        .map(|service| service.service_id);

    page.hashes
        .iter()
        .take(MAX_DISPLAYED_FILES)
        .map(|hash| {
            let thumb = thumbnail_path(controller, hash)
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            let hash_id = hash_id_for(controller, hash);
            let inbox = hash_id
                .map(|hash_id| files_storage::is_inbox(db, hash_id).unwrap_or(false))
                .unwrap_or(false);
            let duplicate = hash_id.is_some_and(|hash_id| duplicate_ids.contains(&hash_id));
            let rating = hash_id
                .zip(rating_service)
                .and_then(|(hash_id, service_id)| {
                    crate::db::ratings::get_rating(db, service_id, hash_id)
                        .ok()
                        .flatten()
                })
                .map(|rating| rating.value());
            let file = file_path(controller, hash)
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            let url = urls_for(controller, hash)
                .into_iter()
                .next()
                .unwrap_or_default();

            serde_json::json!({
                "hash": hex(hash),
                "thumb": thumb,
                "file": file,
                "url": url,
                "inbox": inbox,
                "duplicate": duplicate,
                "rating": rating,
            })
        })
        .collect()
}

pub fn append_hashes(page: &mut QueryPage, hashes: Vec<Vec<u8>>) {
    if let Some(fixed) = &mut page.fixed_hashes {
        for hash in &hashes {
            if !fixed.contains(hash) {
                fixed.push(hash.clone());
            }
        }
    }

    for hash in hashes {
        if !page.hashes.contains(&hash) {
            page.hashes.push(hash);
        }
    }

    page.selected.clear();
    page.status = match page.hashes.len() {
        0 => "0 files".to_string(),
        1 => "1 file".to_string(),
        count => format!("{count} files"),
    };
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
