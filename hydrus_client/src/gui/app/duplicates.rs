//! Duplicate processing page logic.
//!
//! The expensive shape search already happens during import; this module turns
//! the stored potential pairs into a filter queue and applies the user's merge,
//! false-positive and skip decisions.

use std::collections::{BTreeMap, BTreeSet};

use hydrus_core::hydrus_constants::{ContentStatus, ServiceType, TimestampType};

use crate::controller::Controller;
use crate::db::{
    duplicates, files_metadata, files_storage, files_timestamps, mappings, master, notes, ratings,
    similar_files, url_map,
};
use crate::duplicates::merge_options::{
    ArchiveSyncAction, DuplicateContentMergeOptions, MergeAction,
};
use crate::gui::app::auto_resolution::{self, AutoDecision, DuplicateReviewState, PreparedRule};
use crate::gui::app::model::{
    DupePairSort, DupeSearchType, DuplicatePairView, DuplicateSearchContext, DuplicatesPage,
    PixelDupesPreference,
};
use crate::gui::app::{AppState, UiCommand, query, render};
use serde_json::{Value, json};

const MIN_BATCH_SIZE: i64 = 5;
const MAX_BATCH_SIZE: i64 = 1024;

pub fn refresh(state: &mut AppState, page_id: &str) -> Vec<UiCommand> {
    let controller = state.controller.clone();
    let rules = state.auto_rules.clone();
    let mut review = std::mem::take(&mut state.duplicate_review);

    let Some(page) = state.find_duplicates_mut(page_id) else {
        state.duplicate_review = review;

        return vec![UiCommand::NotifyMessage(
            "no duplicates page selected".to_string(),
        )];
    };

    let (_, queued) = load_queue(&controller, page, &rules, &mut review);

    state.duplicate_review = review;

    if queued > 0 {
        let _ = auto_resolution::save_state_review(state);
    }

    duplicates_commands(state, page_id)
}

/// Fills a freshly created page without needing the page tree. Returns how
/// many matches were queued for review so the caller can persist them.
pub fn refresh_for(
    controller: &Controller,
    page: &mut DuplicatesPage,
    rules: &[super::auto_resolution::AutoResolutionRule],
    review: &mut DuplicateReviewState,
) -> (usize, usize) {
    load_queue(controller, page, rules, review)
}

pub fn action(state: &mut AppState, page_id: &str, action: &str, payload: &str) -> Vec<UiCommand> {
    let controller = state.controller.clone();
    let rules = state.auto_rules.clone();
    let mut review = std::mem::take(&mut state.duplicate_review);
    let mut queued = 0usize;

    let Some(page) = state.find_duplicates_mut(page_id) else {
        state.duplicate_review = review;

        return vec![UiCommand::NotifyMessage(
            "no duplicates page selected".to_string(),
        )];
    };

    match action {
        "duplicates_skip" => {
            if !page.pairs.is_empty() {
                page.index = (page.index + 1) % page.pairs.len();
                page.status = format!("{} pairs left", page.pairs.len());
            }
        }
        "duplicates_review" => {
            state.duplicate_review = review;

            return auto_resolution::review_open(state);
        }
        "duplicates_refresh" => {
            queued = load_queue(&controller, page, &rules, &mut review).1;
        }
        "duplicates_next_batch" => {
            queued = load_queue(&controller, page, &rules, &mut review).1;
            page.status = format!("next batch; {}", page.status);
        }
        "duplicates_batch_size" => {
            let Ok(requested) = payload.trim().parse::<i64>() else {
                state.duplicate_review = review;

                return vec![UiCommand::NotifyMessage(
                    "the batch size must be a number".to_string(),
                )];
            };
            let size = requested.clamp(MIN_BATCH_SIZE, MAX_BATCH_SIZE);
            let mut options = controller.options();
            options.set_integer("duplicate_filter_max_batch_size", size);
            controller.replace_options(options);
            let _ = controller.save_options();

            queued = load_queue(&controller, page, &rules, &mut review).1;
            page.status = format!("batch size {size}; {}", page.status);
        }
        "duplicates_search_context" => {
            match serde_json::from_str::<DuplicateSearchContext>(payload) {
                Ok(mut context) => {
                    context.normalise();
                    page.search_context = context;
                    queued = load_queue(&controller, page, &rules, &mut review).1;
                }
                Err(error) => {
                    state.duplicate_review = review;

                    return vec![UiCommand::NotifyMessage(format!(
                        "could not read the duplicate search context: {error}"
                    ))];
                }
            }
        }
        "duplicates_keep_a"
        | "duplicates_keep_b"
        | "duplicates_keep_a_keep"
        | "duplicates_keep_b_keep"
        | "duplicates_same" => {
            let Some(pair) = page.current().cloned() else {
                state.duplicate_review = review;

                return vec![UiCommand::NotifyMessage(
                    "no duplicate pair left".to_string(),
                )];
            };

            let (keep, discard, trash, kind) = match action {
                "duplicates_keep_a" => (
                    pair.a.clone(),
                    pair.b.clone(),
                    true,
                    MergeActionKind::Better,
                ),
                "duplicates_keep_b" => (
                    pair.b.clone(),
                    pair.a.clone(),
                    true,
                    MergeActionKind::Better,
                ),
                "duplicates_keep_a_keep" => (
                    pair.a.clone(),
                    pair.b.clone(),
                    false,
                    MergeActionKind::Better,
                ),
                "duplicates_keep_b_keep" => (
                    pair.b.clone(),
                    pair.a.clone(),
                    false,
                    MergeActionKind::Better,
                ),
                _ => (pair.a.clone(), pair.b.clone(), false, MergeActionKind::Same),
            };
            let options = merge_options(&controller, kind);

            match merge_pair(&controller, &keep, &discard, trash, &options) {
                Ok(()) => {
                    page.remove_current();
                    page.status = match (trash, kind) {
                        (true, MergeActionKind::Better) => {
                            format!("kept {}; moved the other file to trash", query::hex(&keep))
                        }
                        (true, MergeActionKind::Same) => {
                            "files marked as duplicates; both kept".to_string()
                        }
                        (false, MergeActionKind::Better) => {
                            format!("kept {}; both files kept", query::hex(&keep))
                        }
                        (false, MergeActionKind::Same) => {
                            "files marked as duplicates; both kept".to_string()
                        }
                    };
                }
                Err(error) => {
                    page.status = format!("merge failed: {error}");
                }
            }
        }
        "duplicates_alternates" => {
            let Some(pair) = page.current().cloned() else {
                state.duplicate_review = review;

                return vec![UiCommand::NotifyMessage(
                    "no duplicate pair left".to_string(),
                )];
            };

            match apply_rule_decision(
                &controller,
                AutoDecision::Alternates,
                false,
                false,
                &pair.a,
                &pair.b,
            ) {
                Ok(()) => {
                    page.remove_current();
                    page.status = "marked as related alternates".to_string();
                }
                Err(error) => {
                    page.status = format!("could not save the decision: {error}");
                }
            }
        }
        "duplicates_custom_action" => {
            let json = custom_action_dialog_json(&controller, page);
            state.duplicate_review = review;

            return vec![UiCommand::DuplicateCustomActionDialog(json)];
        }
        "duplicates_custom" => {
            let Some(pair) = page.current().cloned() else {
                state.duplicate_review = review;

                return vec![UiCommand::NotifyMessage(
                    "no duplicate pair left".to_string(),
                )];
            };

            match apply_custom_action(&controller, &pair, payload) {
                Ok(message) => {
                    page.remove_current();
                    page.status = message;
                }
                Err(error) => {
                    page.status = format!("custom action failed: {error}");
                }
            }
        }
        "duplicates_group_action" => {
            let Some(pair) = page.current().cloned() else {
                state.duplicate_review = review;

                return vec![UiCommand::NotifyMessage(
                    "no duplicate pair left".to_string(),
                )];
            };

            match apply_group_action(
                &controller,
                &pair,
                payload,
                page.search_context.max_distance,
            ) {
                Ok(message) => {
                    queued = load_queue(&controller, page, &rules, &mut review).1;
                    page.status = format!("{message}; {}", page.status);
                }
                Err(error) => {
                    page.status = format!("duplicate action failed: {error}");
                }
            }
        }
        "duplicates_false" => {
            let Some(pair) = page.current().cloned() else {
                state.duplicate_review = review;

                return vec![UiCommand::NotifyMessage(
                    "no duplicate pair left".to_string(),
                )];
            };

            match mark_false_positive(&controller, &pair.a, &pair.b) {
                Ok(()) => {
                    page.remove_current();
                    page.status = "marked as not duplicates".to_string();
                }
                Err(error) => {
                    page.status = format!("could not save the decision: {error}");
                }
            }
        }
        "duplicates_delete_both" => {
            let Some(pair) = page.current().cloned() else {
                state.duplicate_review = review;

                return vec![UiCommand::NotifyMessage(
                    "no duplicate pair left".to_string(),
                )];
            };

            if let Err(error) = delete_both(&controller, &pair.a, &pair.b) {
                page.status = format!("delete failed: {error}");
            } else {
                page.remove_current();
                page.status = "both files moved to trash".to_string();
            }
        }
        _ => {
            let _ = payload;
        }
    }

    state.duplicate_review = review;

    if queued > 0 {
        let _ = auto_resolution::save_state_review(state);
    }

    let mut commands = duplicates_commands(state, page_id);

    if action != "duplicates_skip" && action != "duplicates_refresh" {
        commands.extend(state.refresh_commands());
    }

    commands
}

fn load_queue(
    controller: &Controller,
    page: &mut DuplicatesPage,
    rules: &[super::auto_resolution::AutoResolutionRule],
    review: &mut DuplicateReviewState,
) -> (usize, usize) {
    page.search_context.normalise();

    let max_distance = page.search_context.max_distance;
    let search_type = page.search_context.search_type();
    let pixel_preference = page.search_context.pixel_preference();
    let batch_size = controller
        .options()
        .get_integer("duplicate_filter_max_batch_size")
        .clamp(MIN_BATCH_SIZE, MAX_BATCH_SIZE) as usize;
    let auto_enabled = controller
        .options()
        .get_boolean("duplicates_auto_resolution_enabled");
    let prepared: Vec<PreparedRule> = if auto_enabled {
        rules.iter().map(|rule| rule.prepare(controller)).collect()
    } else {
        Vec::new()
    };
    let search_errors: Vec<String> = prepared
        .iter()
        .filter_map(|rule| rule.search_error().map(str::to_string))
        .fold(Vec::new(), |mut errors, error| {
            if !errors.contains(&error) {
                errors.push(error);
            }

            errors
        });

    let search_1 = match hash_set_for(
        controller,
        &page.search_context.query_1,
        &page.search_context.location_1,
    ) {
        Ok(hashes) => hashes,
        Err(error) => {
            page.pairs.clear();
            page.index = 0;
            page.searched = 0;
            page.matched = 0;
            page.status = format!("duplicate search failed: {error}");

            return (0, 0);
        }
    };
    let search_2 = if search_type == DupeSearchType::DifferentSearches {
        match hash_set_for(
            controller,
            &page.search_context.query_2,
            &page.search_context.location_2,
        ) {
            Ok(hashes) => hashes,
            Err(error) => {
                page.pairs.clear();
                page.index = 0;
                page.searched = 0;
                page.matched = 0;
                page.status = format!("duplicate search failed: {error}");

                return (0, 0);
            }
        }
    } else {
        search_1.clone()
    };

    match duplicates::get_candidate_pairs(controller.db().db(), Some(max_distance)) {
        Ok(pairs) => {
            let total = pairs.len();
            page.searched = total as i64;
            page.pairs.clear();
            page.index = 0;

            let mut applied = 0usize;
            let mut queued = 0usize;
            let mut matched_count = 0i64;
            let mut candidates: Vec<(Vec<u8>, Vec<u8>, i64)> = Vec::new();

            for (smaller_id, larger_id, distance) in pairs {
                let Some(a) = master::get_hash(controller.db().db(), smaller_id)
                    .ok()
                    .flatten()
                else {
                    continue;
                };
                let Some(b) = master::get_hash(controller.db().db(), larger_id)
                    .ok()
                    .flatten()
                else {
                    continue;
                };

                if !pair_matches_search(search_type, &search_1, &search_2, &a, &b) {
                    continue;
                }

                if !pair_matches_pixel_preference(
                    controller,
                    pixel_preference,
                    smaller_id,
                    larger_id,
                ) {
                    continue;
                }

                candidates.push((a, b, distance));
            }

            sort_pairs(controller, &page.search_context, &mut candidates);

            for (a, b, distance) in candidates {
                matched_count += 1;

                let matched = prepared.iter().find_map(|prepared_rule| {
                    let result = prepared_rule.evaluate(controller, &a, &b, distance)?;

                    Some((prepared_rule.rule(), result))
                });

                if let Some((rule, result)) = matched {
                    if rule.operation_mode == "fully automatic" {
                        match apply_rule_decision(
                            controller,
                            result.decision,
                            result.delete_a,
                            result.delete_b,
                            &a,
                            &b,
                        ) {
                            Ok(()) => {
                                applied += 1;
                                continue;
                            }
                            Err(_) => {}
                        }
                    } else if review.queue(rule, &a, &b, distance, result) {
                        queued += 1;
                        continue;
                    }
                }

                if page.pairs.len() < batch_size {
                    page.pairs.push(DuplicatePairView { a, b, distance });
                }
            }

            page.matched = matched_count;

            let batch = page.pairs.len();
            page.status = match total {
                0 => "no potential duplicates".to_string(),
                1 => "1 potential duplicate pair".to_string(),
                count => format!(
                    "{count} potential duplicate pairs (batch of {batch}, max {batch_size})"
                ),
            };

            if total > 0 {
                page.status = format!("{}; {matched_count} match", page.status);
            }

            if applied > 0 {
                page.status = format!("{}; {applied} auto-resolved", page.status);
            }

            if queued > 0 {
                page.status = format!("{}; {queued} pending review", page.status);
            }

            if let Some(error) = search_errors.first() {
                page.status = format!("{}; auto-resolution search failed: {error}", page.status);
            }

            (applied, queued)
        }
        Err(error) => {
            page.pairs.clear();
            page.index = 0;
            page.matched = 0;
            page.status = format!("could not load the duplicate queue: {error}");

            (0, 0)
        }
    }
}

fn hash_set_for(
    controller: &Controller,
    query_text: &str,
    location: &str,
) -> Result<BTreeSet<Vec<u8>>, String> {
    query::hashes_for(controller, query_text, location).map(|hashes| hashes.into_iter().collect())
}

fn pair_matches_search(
    search_type: DupeSearchType,
    search_1: &BTreeSet<Vec<u8>>,
    search_2: &BTreeSet<Vec<u8>>,
    a: &[u8],
    b: &[u8],
) -> bool {
    let a_1 = search_1.contains(a);
    let b_1 = search_1.contains(b);

    match search_type {
        DupeSearchType::OneFile => a_1 || b_1,
        DupeSearchType::BothFiles => a_1 && b_1,
        DupeSearchType::DifferentSearches => {
            (a_1 && search_2.contains(b)) || (search_2.contains(a) && b_1)
        }
    }
}

fn pair_matches_pixel_preference(
    controller: &Controller,
    preference: PixelDupesPreference,
    a_id: i64,
    b_id: i64,
) -> bool {
    if preference == PixelDupesPreference::Allowed {
        return true;
    }

    let db = controller.db().db();
    let a_pixel = similar_files::get_pixel_hash(db, a_id).ok().flatten();
    let b_pixel = similar_files::get_pixel_hash(db, b_id).ok().flatten();
    let is_pixel_duplicate = matches!((a_pixel, b_pixel), (Some(a), Some(b)) if a == b);

    match preference {
        PixelDupesPreference::Required => is_pixel_duplicate,
        PixelDupesPreference::Excluded => !is_pixel_duplicate,
        PixelDupesPreference::Allowed => true,
    }
}

/// Orders the filtered pairs exactly like `PotentialDuplicateMediaResultPairsAndDistances.Sort`.
/// Sizes of unknown files count as 1 so a missing file cannot dominate the order.
fn sort_pairs(
    controller: &Controller,
    context: &DuplicateSearchContext,
    pairs: &mut [(Vec<u8>, Vec<u8>, i64)],
) {
    let sort = context.pair_sort();

    if sort == DupePairSort::Random {
        use rand::seq::SliceRandom;

        pairs.shuffle(&mut rand::thread_rng());

        return;
    }

    let mut sizes: BTreeMap<Vec<u8>, u64> = BTreeMap::new();

    for (a, b, _) in pairs.iter() {
        for hash in [a.as_slice(), b.as_slice()] {
            sizes.entry(hash.to_vec()).or_insert_with(|| {
                query::hash_id_for(controller, hash)
                    .and_then(|hash_id| {
                        files_metadata::get_file_info(controller.db().db(), hash_id)
                            .ok()
                            .flatten()
                    })
                    .map(|file_info| file_info.size)
                    .filter(|size| *size > 0)
                    .unwrap_or(1)
            });
        }
    }

    let size_for = |hash: &[u8]| -> u64 { sizes.get(hash).copied().unwrap_or(1) };

    match sort {
        DupePairSort::Similarity => {
            pairs.sort_by(|first, second| {
                let sort_key = |(a, b, distance): &(Vec<u8>, Vec<u8>, i64)| {
                    let a_size = size_for(a);
                    let b_size = size_for(b);

                    (
                        *distance,
                        a_size.max(b_size) as f64 / a_size.min(b_size) as f64,
                    )
                };

                sort_key(first)
                    .partial_cmp(&sort_key(second))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        DupePairSort::MinFilesize => {
            pairs.sort_by_key(|(a, b, _)| {
                let a_size = size_for(a);
                let b_size = size_for(b);

                (a_size.min(b_size), a_size.max(b_size))
            });
        }
        DupePairSort::MaxFilesize | DupePairSort::Random => {
            pairs.sort_by_key(|(a, b, _)| {
                let a_size = size_for(a);
                let b_size = size_for(b);

                (a_size.max(b_size), a_size.min(b_size))
            });
        }
    }

    if !context.sort_ascending {
        pairs.reverse();
    }
}

/// Applies one reviewed or automatic match. Shared with the review dialog.
pub(crate) fn apply_rule_decision(
    controller: &Controller,
    decision: AutoDecision,
    delete_a: bool,
    delete_b: bool,
    a: &[u8],
    b: &[u8],
) -> Result<(), String> {
    match decision {
        AutoDecision::KeepFirst => {
            let options = merge_options(controller, MergeActionKind::Better);

            merge_pair(controller, a, b, true, &options)
        }
        AutoDecision::KeepSecond => {
            let options = merge_options(controller, MergeActionKind::Better);

            merge_pair(controller, b, a, true, &options)
        }
        AutoDecision::Same => {
            let options = merge_options(controller, MergeActionKind::Same);

            merge_pair(controller, a, b, false, &options)?;

            if delete_a {
                trash_file(controller, a)?;
            }

            if delete_b {
                trash_file(controller, b)?;
            }

            Ok(())
        }
        AutoDecision::Alternates => {
            let db = controller.db().db();
            let a_id = query::hash_id_for(controller, a)
                .ok_or_else(|| "first file is missing".to_string())?;
            let b_id = query::hash_id_for(controller, b)
                .ok_or_else(|| "second file is missing".to_string())?;

            duplicates::create_alternate_group(db, &[a_id, b_id])
                .map_err(|error| error.to_string())?;
            duplicates::remove_potential_pair(db, a_id.min(b_id), a_id.max(b_id))
                .map_err(|error| error.to_string())?;

            if delete_a {
                trash_file(controller, a)?;
            }

            if delete_b {
                trash_file(controller, b)?;
            }

            Ok(())
        }
        AutoDecision::NotDuplicates => mark_false_positive(controller, a, b),
        AutoDecision::DeleteBoth => delete_both(controller, a, b),
    }
}

fn custom_action_dialog_json(controller: &Controller, page: &DuplicatesPage) -> String {
    let mut value = merge_options_value(controller);
    let pair = page.current().map(|pair| {
        json!({
            "distance": pair.distance,
            "a": render::duplicate_media_json(controller, &pair.a),
            "b": render::duplicate_media_json(controller, &pair.b),
        })
    });

    value["page_id"] = json!(page.id);
    value["pair"] = pair.unwrap_or(Value::Null);
    value["duplicate_types"] = json!([
        { "value": "better", "label": "this is better" },
        { "value": "same", "label": "they are the same quality" },
        { "value": "alternate", "label": "they are related alternates" },
        { "value": "false_positive", "label": "they are not related" },
    ]);
    value["better_sides"] = json!([
        { "value": "a", "label": "A is better" },
        { "value": "b", "label": "B is better" },
    ]);
    value["delete_modes"] = json!([
        { "value": "neither", "label": "delete neither" },
        { "value": "a", "label": "delete A" },
        { "value": "b", "label": "delete B" },
        { "value": "both", "label": "delete both" },
    ]);

    value.to_string()
}

fn apply_custom_action(
    controller: &Controller,
    pair: &DuplicatePairView,
    payload: &str,
) -> Result<String, String> {
    let value: Value =
        serde_json::from_str(payload).map_err(|error| format!("invalid custom action: {error}"))?;

    let action = value
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("better");
    let better_side = value
        .get("better_side")
        .and_then(Value::as_str)
        .unwrap_or("a");
    let delete = value
        .get("delete")
        .and_then(Value::as_str)
        .unwrap_or("neither");
    let delete_a = delete == "a" || delete == "both";
    let delete_b = delete == "b" || delete == "both";

    match action {
        "better" => {
            let options: DuplicateContentMergeOptions = value
                .get("better")
                .cloned()
                .ok_or_else(|| "missing merge options".to_string())
                .and_then(|value| {
                    serde_json::from_value(value).map_err(|error| error.to_string())
                })?;

            let (keep, discard) = if better_side == "b" {
                (&pair.b, &pair.a)
            } else {
                (&pair.a, &pair.b)
            };

            merge_pair(controller, keep, discard, false, &options)?;
        }
        "same" => {
            let options: DuplicateContentMergeOptions = value
                .get("same")
                .cloned()
                .ok_or_else(|| "missing merge options".to_string())
                .and_then(|value| {
                    serde_json::from_value(value).map_err(|error| error.to_string())
                })?;

            merge_pair(controller, &pair.a, &pair.b, false, &options)?;
        }
        "alternate" => {
            let db = controller.db().db();
            let a_id = query::hash_id_for(controller, &pair.a)
                .ok_or_else(|| "first file is missing".to_string())?;
            let b_id = query::hash_id_for(controller, &pair.b)
                .ok_or_else(|| "second file is missing".to_string())?;

            duplicates::create_alternate_group(db, &[a_id, b_id])
                .map_err(|error| error.to_string())?;
            duplicates::remove_potential_pair(db, a_id.min(b_id), a_id.max(b_id))
                .map_err(|error| error.to_string())?;
        }
        "false_positive" => {
            mark_false_positive(controller, &pair.a, &pair.b)?;
        }
        _ => return Err(format!("unknown duplicate action '{action}'")),
    }

    if delete_a {
        trash_file(controller, &pair.a)?;
    }

    if delete_b {
        trash_file(controller, &pair.b)?;
    }

    Ok(match action {
        "better" => format!(
            "kept {} better{}",
            query::hex(if better_side == "b" { &pair.b } else { &pair.a }),
            if delete_a || delete_b {
                "; deleted the selected files"
            } else {
                "; both files kept"
            }
        ),
        "same" => "files marked as duplicates".to_string(),
        "alternate" => "marked as related alternates".to_string(),
        _ => "marked as not duplicates".to_string(),
    })
}

fn apply_group_action(
    controller: &Controller,
    pair: &DuplicatePairView,
    payload: &str,
    max_distance: i64,
) -> Result<String, String> {
    let value: Value = serde_json::from_str(payload)
        .map_err(|error| format!("invalid duplicate action: {error}"))?;
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing action".to_string())?;
    let target = value
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or("both");

    let hashes: Vec<&[u8]> = match target {
        "a" => vec![pair.a.as_slice()],
        "b" => vec![pair.b.as_slice()],
        _ => vec![pair.a.as_slice(), pair.b.as_slice()],
    };
    let ids: Vec<i64> = hashes
        .iter()
        .filter_map(|hash| query::hash_id_for(controller, hash))
        .collect();

    if ids.is_empty() {
        return Err("the files are not in the database".to_string());
    }

    let db = controller.db().db();

    match kind {
        "remove_potentials" => {
            let removed = duplicates::delete_potentials_for_hashes(db, &ids)
                .map_err(|error| error.to_string())?;

            Ok(count_message(removed, "potential pair", "removed"))
        }
        "reset_potential_search" => {
            let mut found = 0usize;

            for hash_id in &ids {
                found += duplicates::reset_potential_search(db, *hash_id, max_distance)
                    .map_err(|error| error.to_string())?;
            }

            Ok(count_message(found, "potential pair", "found again"))
        }
        "clear_false_positives" => {
            let expanded = alternate_expanded_ids(db, &ids)?;
            let removed = duplicates::delete_false_positives_for_hashes(db, &expanded)
                .map_err(|error| error.to_string())?;

            Ok(count_message(removed, "false positive", "cleared"))
        }
        "clear_internal_false_positives" => {
            let removed = duplicates::delete_internal_false_positives(db, &ids)
                .map_err(|error| error.to_string())?;

            Ok(count_message(removed, "false positive", "cleared"))
        }
        "remove_from_duplicate_group" => {
            let mut removed = 0usize;

            for hash_id in &ids {
                removed += duplicates::remove_from_duplicate_group(db, *hash_id)
                    .map_err(|error| error.to_string())?;
            }

            Ok(count_message(
                removed,
                "duplicate group membership",
                "removed",
            ))
        }
        "remove_from_alternate_group" => {
            let mut removed = 0usize;

            for hash_id in &ids {
                removed += duplicates::remove_from_alternate_group(db, *hash_id)
                    .map_err(|error| error.to_string())?;
            }

            Ok(count_message(
                removed,
                "alternates group membership",
                "removed",
            ))
        }
        "dissolve_duplicate_group" => {
            let mut removed = 0usize;

            for hash_id in &ids {
                removed += duplicates::remove_from_alternate_group(db, *hash_id)
                    .map_err(|error| error.to_string())?;
                duplicates::delete_potentials_for_hashes(db, &[*hash_id])
                    .map_err(|error| error.to_string())?;

                for media_id in duplicates::get_duplicate_media_ids_for_hash(db, *hash_id)
                    .map_err(|error| error.to_string())?
                {
                    duplicates::delete_duplicate_group(db, media_id)
                        .map_err(|error| error.to_string())?;
                    removed += 1;
                }
            }

            Ok(count_message(
                removed,
                "duplicate relationship",
                "dissolved",
            ))
        }
        "dissolve_alternate_group" => {
            let mut group_ids = BTreeSet::new();

            for hash_id in &ids {
                group_ids.extend(
                    duplicates::get_alternate_group_ids_for_hash(db, *hash_id)
                        .map_err(|error| error.to_string())?,
                );
            }

            let mut removed = 0usize;

            for group_id in group_ids {
                let members = duplicates::get_alternate_group_hashes(db, group_id)
                    .map_err(|error| error.to_string())?;

                for member in &members {
                    duplicates::remove_from_duplicate_group(db, *member)
                        .map_err(|error| error.to_string())?;
                    duplicates::delete_potentials_for_hashes(db, &[*member])
                        .map_err(|error| error.to_string())?;
                }

                duplicates::delete_alternate_group(db, group_id)
                    .map_err(|error| error.to_string())?;
                removed += 1;
            }

            Ok(count_message(removed, "alternates group", "dissolved"))
        }
        _ => Err(format!("unknown duplicate action '{kind}'")),
    }
}

fn alternate_expanded_ids(
    db: &hydrus_core::hydrus_db_base::DBBase,
    ids: &[i64],
) -> Result<Vec<i64>, String> {
    let mut expanded: BTreeSet<i64> = ids.iter().copied().collect();

    for hash_id in ids {
        for group_id in duplicates::get_alternate_group_ids_for_hash(db, *hash_id)
            .map_err(|error| error.to_string())?
        {
            expanded.extend(
                duplicates::get_alternate_group_hashes(db, group_id)
                    .map_err(|error| error.to_string())?,
            );
        }
    }

    Ok(expanded.into_iter().collect())
}

fn count_message(count: usize, noun: &str, verb: &str) -> String {
    if count == 0 {
        format!("no {noun} to {verb}")
    } else if count == 1 {
        format!("1 {noun} {verb}")
    } else {
        format!("{count} {noun}s {verb}")
    }
}

fn trash_file(controller: &Controller, hash: &[u8]) -> Result<(), String> {
    let db = controller.db().db();
    let trash =
        query::trash_service_id(controller).ok_or_else(|| "no trash service".to_string())?;
    let my_files = query::local_file_service_id(controller, "my files")
        .ok_or_else(|| "no local file domain".to_string())?;
    let hash_id = query::hash_id_for(controller, hash)
        .ok_or_else(|| "file is not in the database".to_string())?;

    files_storage::add_files(
        db,
        trash,
        ContentStatus::Current,
        &[hash_id],
        Some(hydrus_core::hydrus_time::get_now()),
    )
    .map_err(|error| error.to_string())?;
    files_storage::remove_files(db, my_files, ContentStatus::Current, &[hash_id])
        .map_err(|error| error.to_string())?;
    let _ = files_storage::set_inbox(db, &[hash_id], false);

    Ok(())
}

const MERGE_OPTIONS_GROUP: &str = "duplicate_action_options";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MergeActionKind {
    Better,
    Same,
}

impl MergeActionKind {
    fn key(self) -> &'static str {
        match self {
            Self::Better => "better",
            Self::Same => "same",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Better => "this is better",
            Self::Same => "same quality",
        }
    }

    fn defaults(self) -> DuplicateContentMergeOptions {
        match self {
            Self::Better => DuplicateContentMergeOptions::better_default(),
            Self::Same => DuplicateContentMergeOptions::same_default(),
        }
    }
}

fn merge_options(controller: &Controller, kind: MergeActionKind) -> DuplicateContentMergeOptions {
    controller
        .options()
        .get_group_value(MERGE_OPTIONS_GROUP, kind.key())
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_else(|| kind.defaults())
}

fn merge_options_json_for(controller: &Controller, kind: MergeActionKind) -> serde_json::Value {
    serde_json::to_value(merge_options(controller, kind)).unwrap_or_default()
}

pub fn merge_options_json(state: &AppState) -> String {
    merge_options_value(&state.controller).to_string()
}

fn merge_options_value(controller: &Controller) -> Value {
    let manager = controller.services();
    let tag_services: Vec<String> = manager
        .get_services(Some(&[ServiceType::LocalTag, ServiceType::TagRepository]))
        .into_iter()
        .map(|service| service.name.clone())
        .collect();
    let rating_services: Vec<String> = manager
        .get_services(Some(&[
            ServiceType::LocalRatingLike,
            ServiceType::LocalRatingNumerical,
            ServiceType::LocalRatingIncDec,
        ]))
        .into_iter()
        .map(|service| service.name.clone())
        .collect();

    let actions = [
        MergeAction::Copy,
        MergeAction::Move,
        MergeAction::TwoWayMerge,
        MergeAction::None,
    ]
    .into_iter()
    .map(|action| {
        json!({
            "value": action.as_str(),
            "label": action.label(),
        })
    })
    .collect::<Vec<_>>();

    let archive_actions = [
        ArchiveSyncAction::None,
        ArchiveSyncAction::IfOneDoBoth,
        ArchiveSyncAction::AlwaysBoth,
    ]
    .into_iter()
    .map(|action| {
        json!({
            "value": action.as_str(),
            "label": action.label(),
        })
    })
    .collect::<Vec<_>>();

    json!({
        "better": merge_options_json_for(controller, MergeActionKind::Better),
        "same": merge_options_json_for(controller, MergeActionKind::Same),
        "tag_services": tag_services,
        "rating_services": rating_services,
        "actions": actions,
        "archive_actions": archive_actions,
        "summary": {
            "better": merge_options(controller, MergeActionKind::Better).get_merge_summary(),
            "same": merge_options(controller, MergeActionKind::Same).get_merge_summary(),
        },
    })
}

fn save_merge_options(
    controller: &Controller,
    kind: MergeActionKind,
    options: DuplicateContentMergeOptions,
) {
    let mut client_options = controller.options();
    client_options.set_group_value(
        MERGE_OPTIONS_GROUP,
        kind.key(),
        serde_json::to_value(options).unwrap_or_default(),
    );
    controller.replace_options(client_options);
    let _ = controller.save_options();
}

/// Saves one merge options page from the editor dialog. This runs without an
/// active duplicates page so the dialog can be opened from anywhere.
pub fn save_merge_options_action(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let controller = state.controller.clone();

    let value: serde_json::Value = match serde_json::from_str(payload) {
        Ok(value) => value,
        Err(error) => {
            return vec![UiCommand::NotifyMessage(format!(
                "could not read the merge options: {error}"
            ))];
        }
    };

    let kind = match value.get("action").and_then(|value| value.as_str()) {
        Some("same") => MergeActionKind::Same,
        _ => MergeActionKind::Better,
    };

    let options = value
        .get("options")
        .cloned()
        .ok_or_else(|| "missing options".to_string())
        .and_then(|value| {
            serde_json::from_value::<DuplicateContentMergeOptions>(value)
                .map_err(|error| error.to_string())
        });

    match options {
        Ok(options) => {
            save_merge_options(&controller, kind, options);

            vec![
                UiCommand::NotifyMessage(format!("{} merge options saved", kind.label())),
                UiCommand::Status(render::status_json(state)),
            ]
        }
        Err(error) => vec![UiCommand::NotifyMessage(format!(
            "could not read the merge options: {error}"
        ))],
    }
}

fn merge_pair(
    controller: &Controller,
    keep: &[u8],
    discard: &[u8],
    trash_discard: bool,
    options: &DuplicateContentMergeOptions,
) -> Result<(), String> {
    let db = controller.db().db();

    let keep_id = query::hash_id_for(controller, keep)
        .ok_or_else(|| "kept file is not in the database".to_string())?;
    let discard_id = query::hash_id_for(controller, discard)
        .ok_or_else(|| "other file is not in the database".to_string())?;

    merge_tag_services(controller, keep_id, discard_id, options);

    for service in controller.services().get_services(Some(&[
        ServiceType::LocalRatingLike,
        ServiceType::LocalRatingNumerical,
        ServiceType::LocalRatingIncDec,
    ])) {
        let action = options.rating_action(&service.name);

        if action != MergeAction::None {
            merge_rating(
                db,
                service.service_id,
                service.service_type,
                keep_id,
                discard_id,
                action,
            );
        }
    }

    merge_urls(db, keep_id, discard_id, options.sync_urls_action);
    merge_notes(db, keep_id, discard_id, options.sync_notes_action);
    merge_modified_date(
        db,
        keep_id,
        discard_id,
        options.sync_file_modified_date_action,
    );
    merge_archive_status(db, keep_id, discard_id, options.sync_archive_action);

    if trash_discard {
        if let Some(trash_service) = query::trash_service_id(controller) {
            let _ = files_storage::add_files(
                db,
                trash_service,
                ContentStatus::Current,
                &[discard_id],
                Some(hydrus_core::hydrus_time::get_now()),
            );
        }

        if let Some(my_files) = query::local_file_service_id(controller, "my files") {
            let _ =
                files_storage::remove_files(db, my_files, ContentStatus::Current, &[discard_id]);
        }

        let _ = files_storage::set_inbox(db, &[discard_id], false);
    }

    let _ = duplicates::create_alternate_group(db, &[keep_id, discard_id]);
    let _ = duplicates::remove_potential_pair(db, keep_id.min(discard_id), keep_id.max(discard_id));

    Ok(())
}

fn merge_tag_services(
    controller: &Controller,
    keep_id: i64,
    discard_id: i64,
    options: &DuplicateContentMergeOptions,
) {
    let db = controller.db().db();

    for service in controller
        .services()
        .get_services(Some(&[ServiceType::LocalTag, ServiceType::TagRepository]))
    {
        let action = options.tag_action(&service.name);

        if action == MergeAction::None {
            continue;
        }

        copy_tags(db, service.service_id, keep_id, discard_id);

        if action != MergeAction::Copy {
            copy_tags(db, service.service_id, discard_id, keep_id);
        }

        if action == MergeAction::Move {
            let discard_tag_ids =
                mappings::get_tag_ids(db, service.service_id, ContentStatus::Current, discard_id)
                    .unwrap_or_default();
            let _ = mappings::remove_mappings(
                db,
                service.service_id,
                ContentStatus::Current,
                discard_id,
                &discard_tag_ids,
            );
        }
    }
}

fn copy_tags(db: &hydrus_core::hydrus_db_base::DBBase, service_id: i64, target: i64, source: i64) {
    let target_tags: BTreeSet<i64> =
        mappings::get_tag_ids(db, service_id, ContentStatus::Current, target)
            .unwrap_or_default()
            .into_iter()
            .collect();
    let source_tags =
        mappings::get_tag_ids(db, service_id, ContentStatus::Current, source).unwrap_or_default();
    let missing: Vec<i64> = source_tags
        .into_iter()
        .filter(|tag_id| !target_tags.contains(tag_id))
        .collect();

    if !missing.is_empty() {
        let _ = mappings::add_mappings(db, service_id, ContentStatus::Current, target, &missing);
    }
}

fn merge_rating(
    db: &hydrus_core::hydrus_db_base::DBBase,
    service_id: i64,
    service_type: ServiceType,
    keep_id: i64,
    discard_id: i64,
    action: MergeAction,
) {
    let keep_rating = ratings::get_rating(db, service_id, keep_id).ok().flatten();
    let discard_rating = ratings::get_rating(db, service_id, discard_id)
        .ok()
        .flatten();

    if service_type == ServiceType::LocalRatingIncDec {
        let keep_value = keep_rating.map(|rating| rating.value() as i64).unwrap_or(0);
        let discard_value = discard_rating
            .map(|rating| rating.value() as i64)
            .unwrap_or(0);

        match action {
            MergeAction::Copy | MergeAction::Move => {
                if discard_value != 0 {
                    let _ = ratings::set_incdec_rating(
                        db,
                        service_id,
                        keep_id,
                        keep_value + discard_value,
                    );
                }
            }
            MergeAction::TwoWayMerge => {
                if discard_value != 0 {
                    let _ = ratings::set_incdec_rating(
                        db,
                        service_id,
                        keep_id,
                        keep_value + discard_value,
                    );
                }

                if keep_value != 0 && keep_rating != discard_rating {
                    let _ = ratings::set_incdec_rating(
                        db,
                        service_id,
                        discard_id,
                        keep_value + discard_value,
                    );
                }
            }
            MergeAction::None => {}
        }

        if action == MergeAction::Move {
            // The moved value lives on the kept file now.
            let _ = ratings::delete_rating(db, service_id, discard_id);
        }

        return;
    }

    let better = match (keep_rating, discard_rating) {
        (None, Some(rating)) => Some(rating),
        (Some(current), Some(rating)) if rating.value() > current.value() => Some(rating),
        _ => None,
    };

    if let Some(rating) = better {
        let _ = ratings::set_rating(db, service_id, keep_id, rating);
    }

    if action == MergeAction::TwoWayMerge {
        match (keep_rating, discard_rating) {
            (Some(current), None) => {
                let _ = ratings::set_rating(db, service_id, discard_id, current);
            }
            (Some(current), Some(rating)) if current.value() > rating.value() => {
                let _ = ratings::set_rating(db, service_id, discard_id, current);
            }
            _ => {}
        }
    }

    if action == MergeAction::Move {
        let _ = ratings::delete_rating(db, service_id, discard_id);
    }
}

fn merge_urls(
    db: &hydrus_core::hydrus_db_base::DBBase,
    keep_id: i64,
    discard_id: i64,
    action: MergeAction,
) {
    if action == MergeAction::None {
        return;
    }

    let keep_urls: BTreeSet<String> = url_map::get_urls(db, keep_id)
        .unwrap_or_default()
        .into_iter()
        .collect();
    let discard_urls = url_map::get_urls(db, discard_id).unwrap_or_default();

    for url in &discard_urls {
        if !keep_urls.contains(url) {
            let _ = url_map::add_url(db, keep_id, url);
        }
    }

    if action == MergeAction::TwoWayMerge {
        let discard_set: BTreeSet<String> = discard_urls.iter().cloned().collect();

        for url in &keep_urls {
            if !discard_set.contains(url) {
                let _ = url_map::add_url(db, discard_id, url);
            }
        }
    }

    if action == MergeAction::Move {
        for url in &discard_urls {
            let _ = url_map::delete_url(db, discard_id, url);
        }
    }
}

fn merge_notes(
    db: &hydrus_core::hydrus_db_base::DBBase,
    keep_id: i64,
    discard_id: i64,
    action: MergeAction,
) {
    if action == MergeAction::None {
        return;
    }

    let keep_notes = notes::get_notes(db, keep_id).unwrap_or_default();
    let discard_notes = notes::get_notes(db, discard_id).unwrap_or_default();

    for (name, note) in &discard_notes {
        if !keep_notes.contains_key(name) {
            let _ = notes::set_note(db, keep_id, name, note);
        }
    }

    if action == MergeAction::TwoWayMerge {
        for (name, note) in &keep_notes {
            if !discard_notes.contains_key(name) {
                let _ = notes::set_note(db, discard_id, name, note);
            }
        }
    }

    if action == MergeAction::Move {
        for name in discard_notes.keys() {
            let _ = notes::delete_note(db, discard_id, name);
        }
    }
}

fn merge_modified_date(
    db: &hydrus_core::hydrus_db_base::DBBase,
    keep_id: i64,
    discard_id: i64,
    action: MergeAction,
) {
    if action == MergeAction::None {
        return;
    }

    let keep_time = files_timestamps::get_file_timestamp(db, keep_id, TimestampType::ModifiedFile)
        .ok()
        .flatten();
    let discard_time =
        files_timestamps::get_file_timestamp(db, discard_id, TimestampType::ModifiedFile)
            .ok()
            .flatten();

    let Some(discard_time) = discard_time else {
        return;
    };

    if action == MergeAction::TwoWayMerge {
        let earliest = match keep_time {
            Some(keep_time) => keep_time.min(discard_time),
            None => discard_time,
        };

        let _ = files_timestamps::set_file_timestamp(
            db,
            keep_id,
            TimestampType::ModifiedFile,
            Some(earliest),
        );

        let _ = files_timestamps::set_file_timestamp(
            db,
            discard_id,
            TimestampType::ModifiedFile,
            Some(earliest),
        );

        return;
    }

    if keep_time.is_none_or(|keep_time| discard_time < keep_time) {
        let _ = files_timestamps::set_file_timestamp(
            db,
            keep_id,
            TimestampType::ModifiedFile,
            Some(discard_time),
        );
    }
}

fn merge_archive_status(
    db: &hydrus_core::hydrus_db_base::DBBase,
    keep_id: i64,
    discard_id: i64,
    action: ArchiveSyncAction,
) {
    if action == ArchiveSyncAction::None {
        return;
    }

    let keep_archived = !files_storage::is_inbox(db, keep_id).unwrap_or(true);
    let discard_archived = !files_storage::is_inbox(db, discard_id).unwrap_or(true);

    let archive_both = match action {
        ArchiveSyncAction::None => false,
        ArchiveSyncAction::IfOneDoBoth => keep_archived || discard_archived,
        ArchiveSyncAction::AlwaysBoth => true,
    };

    if archive_both {
        let _ = files_storage::set_inbox(db, &[keep_id, discard_id], false);
        let now = hydrus_core::hydrus_time::get_now();
        let _ =
            files_timestamps::set_file_timestamp(db, keep_id, TimestampType::Archived, Some(now));
        let _ = files_timestamps::set_file_timestamp(
            db,
            discard_id,
            TimestampType::Archived,
            Some(now),
        );
    }
}

fn mark_false_positive(controller: &Controller, a: &[u8], b: &[u8]) -> Result<(), String> {
    let db = controller.db().db();

    let a_id =
        query::hash_id_for(controller, a).ok_or_else(|| "first file is missing".to_string())?;
    let b_id =
        query::hash_id_for(controller, b).ok_or_else(|| "second file is missing".to_string())?;

    duplicates::add_false_positive(db, a_id.min(b_id), a_id.max(b_id))
        .map_err(|error| error.to_string())?;
    duplicates::remove_potential_pair(db, a_id.min(b_id), a_id.max(b_id))
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn delete_both(controller: &Controller, a: &[u8], b: &[u8]) -> Result<(), String> {
    let db = controller.db().db();
    let trash =
        query::trash_service_id(controller).ok_or_else(|| "no trash service".to_string())?;
    let my_files = query::local_file_service_id(controller, "my files")
        .ok_or_else(|| "no local file domain".to_string())?;

    for hash in [a, b] {
        let hash_id = query::hash_id_for(controller, hash)
            .ok_or_else(|| "file is not in the database".to_string())?;

        let _ = files_storage::add_files(
            db,
            trash,
            ContentStatus::Current,
            &[hash_id],
            Some(hydrus_core::hydrus_time::get_now()),
        );
        let _ = files_storage::remove_files(db, my_files, ContentStatus::Current, &[hash_id]);
        let _ = files_storage::set_inbox(db, &[hash_id], false);
    }

    let a_id = query::hash_id_for(controller, a).unwrap_or(0);
    let b_id = query::hash_id_for(controller, b).unwrap_or(0);
    let _ = duplicates::remove_potential_pair(db, a_id.min(b_id), a_id.max(b_id));

    Ok(())
}

/// Flattens the current batch of duplicate pairs into the unique list of file
/// hashes that the Python media results panel renders as thumbnails. The order
/// follows the (already sorted) pairs, with duplicate hashes collapsed so a file
/// that participates in several pairs is shown only once.
pub fn pair_thumbnail_hashes(page: &DuplicatesPage) -> Vec<Vec<u8>> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();

    for pair in &page.pairs {
        for hash in [&pair.a, &pair.b] {
            if seen.insert(hash.clone()) {
                result.push(hash.clone());
            }
        }
    }

    result
}

/// Empty-state text used by the media results panel when no pairs are queued.
pub fn duplicates_media_empty_status() -> &'static str {
    "the potential duplicate queue is empty"
}

/// Count string for the media results panel status line.
pub fn duplicates_media_count_status(count: usize) -> String {
    match count {
        0 => "no potential duplicate pairs".to_string(),
        1 => "1 potential duplicate pair".to_string(),
        n => format!("{n} potential duplicate pairs"),
    }
}

/// Split a list of candidate pairs into batches of at most `batch_size`.
pub fn split_into_batches<T: Clone>(items: &[T], batch_size: usize) -> Vec<Vec<T>> {
    if items.is_empty() {
        return Vec::new();
    }
    let size = if batch_size == 0 { items.len() } else { batch_size };
    items.chunks(size).map(|chunk| chunk.to_vec()).collect()
}

/// Fraction of eligible files that have been searched at the current distance.
pub fn preparation_progress(searched: i64, total: i64) -> f64 {
    if total <= 0 {
        return 0.0;
    }
    let ratio = searched as f64 / total as f64;
    ratio.clamp(0.0, 1.0)
}

/// Progress as a percentage string, matching Hydrus' "99.9%" style.
pub fn preparation_progress_percent(searched: i64, total: i64) -> String {
    if total <= 0 {
        return "0.0%".to_string();
    }
    let percent = preparation_progress(searched, total) * 100.0;
    if percent >= 100.0 {
        "99.9%".to_string()
    } else {
        format!("{percent:.1}%")
    }
}

/// Status text for the "num searched" gauge on the preparation tab.
pub fn preparation_search_status(searched: i64, total: i64) -> String {
    if total <= 0 {
        return "All potential duplicates found at this distance.".to_string();
    }
    if searched <= 0 {
        "Have not yet searched at this distance.".to_string()
    } else if searched >= total {
        "All potential duplicates found at this distance.".to_string()
    } else {
        format!("Searched {} of {} files at this distance.", searched, total)
    }
}

/// Name shown on the preparation tab, including the "(xx.x% done)" suffix.
pub fn preparation_tab_name(searched: i64, total: i64, hide_when_caught_up: bool) -> String {
    if total <= 0 {
        return "preparation".to_string();
    }
    let percent_done = preparation_progress(searched, total);
    let not_all_searched = searched < total;
    if !not_all_searched {
        return "preparation".to_string();
    }
    let show_percentage = !(hide_when_caught_up && percent_done > 0.99);
    if !show_percentage {
        return "preparation".to_string();
    }
    format!("preparation ({} done)", preparation_progress_percent(searched, total))
}

/// Whether a completed batch may be auto-committed without confirmation.
pub fn should_auto_commit(decision_count: usize, threshold: Option<i64>) -> bool {
    match threshold {
        Some(limit) if limit > 0 => decision_count <= limit as usize,
        _ => false,
    }
}

/// Status string for the not-prepared / preparing / prepared states of a batch.
pub fn preparation_batch_status(total: usize, batch: usize, max_batch: usize) -> String {
    if total == 0 {
        "no potential duplicates".to_string()
    } else if batch == 0 {
        "preparing".to_string()
    } else if batch >= total {
        "prepared".to_string()
    } else {
        format!("{total} potential duplicate pairs (batch of {batch}, max {max_batch})")
    }
}

fn duplicates_commands(state: &AppState, page_id: &str) -> Vec<UiCommand> {
    let Some(page) = state.find_duplicates(page_id) else {
        return vec![];
    };

    let mut duplicates_value: serde_json::Value =
        serde_json::from_str(&render::duplicates_json(&state.controller, page)).unwrap_or_default();
    duplicates_value["auto_rules"] = serde_json::json!(
        state
            .auto_rules
            .iter()
            .map(|rule| serde_json::json!({
                "name": rule.name,
                "paused": rule.paused,
                "search_summary": rule.search_summary(),
            }))
            .collect::<Vec<_>>()
    );

    vec![
        UiCommand::UpdateDuplicates(
            page_id.to_string(),
            duplicates_value.to_string(),
        ),
        UiCommand::Status(render::status_json(state)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        duplicates, files_metadata, files_storage, files_timestamps, mappings, master_tags, notes,
        ratings, similar_files, url_map,
    };
    use crate::gui::app::auto_resolution::{AutoResolutionRule, Comparator};
    use hydrus_core::files::hydrus_file_handling::FileInfo;
    use hydrus_core::hydrus_constants::Mime;

    fn temp_directory(label: &str) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "hydrus_duplicates_{label}_{}_{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    fn test_controller(directory: &std::path::Path) -> Controller {
        let _ = crate::client_options::register_serialisable_type();
        let controller = Controller::new(directory).unwrap();
        controller.init_model().unwrap();

        controller
    }

    fn add_pair(controller: &Controller, seed: u8, distance: i64) {
        add_sized_pair(controller, seed, 1024, 1024, distance);
    }

    fn add_sized_pair(
        controller: &Controller,
        seed: u8,
        first_size: u64,
        second_size: u64,
        distance: i64,
    ) -> (Vec<u8>, Vec<u8>, i64, i64) {
        let db = controller.db().db();
        let first = [seed; 32];
        let second = [seed.wrapping_add(1); 32];
        let first_id = crate::db::master::get_hash_id(db, &first).unwrap();
        let second_id = crate::db::master::get_hash_id(db, &second).unwrap();
        let my_files = query::local_file_service_id(controller, "my files").unwrap();

        for (id, size) in [(first_id, first_size), (second_id, second_size)] {
            files_metadata::set_file_info(
                db,
                id,
                &FileInfo {
                    size,
                    mime: Mime::Png,
                    width: Some(640),
                    height: Some(480),
                    duration_ms: None,
                    num_frames: None,
                    has_audio: false,
                    num_words: None,
                },
            )
            .unwrap();

            files_storage::add_files(
                db,
                my_files,
                ContentStatus::Current,
                &[id],
                Some(hydrus_core::hydrus_time::get_now()),
            )
            .unwrap();
        }

        duplicates::add_potential_pair(
            db,
            first_id.min(second_id),
            first_id.max(second_id),
            distance,
        )
        .unwrap();

        (first.to_vec(), second.to_vec(), first_id, second_id)
    }

    fn add_tag(controller: &Controller, hash: &[u8], tag: &str) {
        let db = controller.db().db();
        let tag_service_id = query::tag_service_id(controller).unwrap();
        let hash_id = query::hash_id_for(controller, hash).unwrap();
        let tag_id = master_tags::get_tag_id(db, tag).unwrap();

        mappings::add_mappings(
            db,
            tag_service_id,
            ContentStatus::Current,
            hash_id,
            &[tag_id],
        )
        .unwrap();
    }

    fn set_pixel_hash(controller: &Controller, hash_id: i64, pixel_hash: &[u8]) {
        similar_files::set_pixel_hash(controller.db().db(), hash_id, pixel_hash).unwrap();
    }

    #[test]
    fn batches_are_limited_and_the_size_is_persisted_in_options() {
        let directory = temp_directory("batch");
        let controller = test_controller(&directory);

        let mut options = controller.options();
        options.set_integer("duplicate_filter_max_batch_size", 5);
        controller.replace_options(options);
        controller.save_options().unwrap();

        for seed in [1u8, 3, 5, 7, 9, 11] {
            add_pair(&controller, seed, 2);
        }

        let mut page = DuplicatesPage::new("dup-batch");
        refresh_for(
            &controller,
            &mut page,
            &[],
            &mut DuplicateReviewState::default(),
        );

        assert_eq!(page.searched, 6);
        assert_eq!(page.pairs.len(), 5);
        assert!(page.status.contains("batch of 5"), "{}", page.status);

        let value: serde_json::Value =
            serde_json::from_str(&render::duplicates_json(&controller, &page)).unwrap();
        assert_eq!(value["batch_size"], 5);
        assert_eq!(value["total"], 5);
        assert_eq!(value["searched"], 6);

        // Deciding a pair removes it, so the next preparation shows the rest.
        let pair = page.pairs[0].clone();
        let db = controller.db().db();
        let a = query::hash_id_for(&controller, &pair.a).unwrap();
        let b = query::hash_id_for(&controller, &pair.b).unwrap();
        duplicates::remove_potential_pair(db, a.min(b), a.max(b)).unwrap();

        refresh_for(
            &controller,
            &mut page,
            &[],
            &mut DuplicateReviewState::default(),
        );
        assert_eq!(page.searched, 5);
        assert_eq!(page.pairs.len(), 5);

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn the_queue_can_be_empty() {
        let directory = temp_directory("empty");
        let controller = test_controller(&directory);
        let mut page = DuplicatesPage::new("dup-empty");

        refresh_for(
            &controller,
            &mut page,
            &[],
            &mut DuplicateReviewState::default(),
        );

        assert!(page.pairs.is_empty());
        assert_eq!(page.searched, 0);
        assert_eq!(page.status, "no potential duplicates");

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn fully_automatic_rules_apply_and_semi_automatic_rules_queue() {
        let directory = temp_directory("auto");
        let controller = test_controller(&directory);
        let mut options = controller.options();
        options.set_boolean("duplicates_auto_resolution_enabled", true);
        controller.replace_options(options);
        controller.save_options().unwrap();

        let (_, _, first_id, second_id) = add_sized_pair(&controller, 21, 1_000, 5_000, 1);
        let rule = AutoResolutionRule {
            name: "big wins".to_string(),
            operation_mode: "fully automatic".to_string(),
            action: "better".to_string(),
            pair_selector: vec![Comparator::Relative {
                property: "size".to_string(),
                operator: ">".to_string(),
                multiplier: 1.0,
                delta: 0.0,
            }],
            ..AutoResolutionRule::default()
        };

        let mut page = DuplicatesPage::new("dup-auto");
        let mut review = DuplicateReviewState::default();
        let (applied, queued) = refresh_for(&controller, &mut page, &[rule.clone()], &mut review);

        assert_eq!((applied, queued), (1, 0));
        assert!(review.pending.is_empty());
        assert!(
            duplicates::get_candidate_pairs(controller.db().db(), None)
                .unwrap()
                .is_empty()
        );
        assert!(page.pairs.is_empty());

        // The worse file moved to trash; the better one stayed in my files.
        let db = controller.db().db();
        let my_files = query::local_file_service_id(&controller, "my files").unwrap();
        let trash = query::trash_service_id(&controller).unwrap();
        let kept = first_id.max(second_id);
        let discarded = first_id.min(second_id);

        assert!(
            files_storage::get_hash_ids(db, my_files, ContentStatus::Current)
                .unwrap()
                .contains(&kept)
        );
        assert!(
            !files_storage::get_hash_ids(db, my_files, ContentStatus::Current)
                .unwrap()
                .contains(&discarded)
        );
        assert!(
            files_storage::get_hash_ids(db, trash, ContentStatus::Current)
                .unwrap()
                .contains(&discarded)
        );

        // The same rule in semi-automatic mode leaves the database alone.
        let (_, _, _, _) = add_sized_pair(&controller, 31, 1_000, 5_000, 1);
        let mut semi = rule.clone();
        semi.name = "big review".to_string();
        semi.operation_mode = "semi-automatic".to_string();
        let mut page = DuplicatesPage::new("dup-semi");
        let mut review = DuplicateReviewState::default();
        let (applied, queued) = refresh_for(&controller, &mut page, &[semi], &mut review);

        assert_eq!((applied, queued), (0, 1));
        assert_eq!(review.pending.len(), 1);
        assert_eq!(
            duplicates::get_candidate_pairs(controller.db().db(), None)
                .unwrap()
                .len(),
            1
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn search_context_filters_by_distance_and_tags() {
        let directory = temp_directory("search-context");
        let controller = test_controller(&directory);
        let (first_a, first_b, _, _) = add_sized_pair(&controller, 41, 1_000, 1_000, 2);
        let _ = add_sized_pair(&controller, 61, 1_000, 1_000, 2);
        let _ = add_sized_pair(&controller, 51, 1_000, 1_000, 10);

        add_tag(&controller, &first_a, "blue eyes");
        add_tag(&controller, &first_b, "red hair");

        let mut page = DuplicatesPage::new("dup-search");
        page.search_context.max_distance = 4;
        page.search_context.query_1 = "blue eyes".to_string();

        let mut review = DuplicateReviewState::default();
        refresh_for(&controller, &mut page, &[], &mut review);

        assert_eq!(page.searched, 2);
        assert_eq!(page.matched, 1);
        assert_eq!(page.pairs.len(), 1);
        assert_eq!(page.pairs[0].a, first_a);

        // Both files must match the one search.
        page.search_context.dupe_search_type = "both_files".to_string();
        refresh_for(&controller, &mut page, &[], &mut review);
        assert_eq!(page.matched, 0);
        assert!(page.pairs.is_empty());

        // The two files may instead match different searches.
        page.search_context.dupe_search_type = "different_searches".to_string();
        page.search_context.query_2 = "red hair".to_string();
        refresh_for(&controller, &mut page, &[], &mut review);
        assert_eq!(page.matched, 1);
        assert_eq!(page.pairs[0].a, first_a);
        assert_eq!(page.pairs[0].b, first_b);

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn pixel_preference_requires_excludes_or_allows_pixel_dupes() {
        let directory = temp_directory("pixel-dupes");
        let controller = test_controller(&directory);
        let (pixel_a, pixel_b, pixel_a_id, pixel_b_id) =
            add_sized_pair(&controller, 71, 1_000, 1_000, 1);
        let (other_a, other_b, other_a_id, other_b_id) =
            add_sized_pair(&controller, 81, 1_000, 1_000, 1);

        let same_pixel_hash = vec![0xAA; 16];
        set_pixel_hash(&controller, pixel_a_id, &same_pixel_hash);
        set_pixel_hash(&controller, pixel_b_id, &same_pixel_hash);
        set_pixel_hash(&controller, other_a_id, &vec![0xBB; 16]);
        set_pixel_hash(&controller, other_b_id, &vec![0xCC; 16]);

        let mut page = DuplicatesPage::new("dup-pixel");
        let mut review = DuplicateReviewState::default();

        page.search_context.pixel_dupes = "required".to_string();
        refresh_for(&controller, &mut page, &[], &mut review);
        assert_eq!(page.matched, 1);
        assert_eq!(page.pairs[0].a, pixel_a);
        assert_eq!(page.pairs[0].b, pixel_b);

        page.search_context.pixel_dupes = "excluded".to_string();
        refresh_for(&controller, &mut page, &[], &mut review);
        assert_eq!(page.matched, 1);
        assert_eq!(page.pairs[0].a, other_a);
        assert_eq!(page.pairs[0].b, other_b);

        page.search_context.pixel_dupes = "allowed".to_string();
        refresh_for(&controller, &mut page, &[], &mut review);
        assert_eq!(page.matched, 2);
        assert_eq!(page.pairs.len(), 2);

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn duplicate_pairs_follow_the_sort_choice() {
        let directory = temp_directory("sort");
        let controller = test_controller(&directory);
        let (small_a, _, _, _) = add_sized_pair(&controller, 21, 100, 100, 2);
        let (medium_a, _, _, _) = add_sized_pair(&controller, 31, 1_000, 1_000, 2);
        let (large_a, _, _, _) = add_sized_pair(&controller, 41, 5_000, 4_000, 2);

        let mut page = DuplicatesPage::new("dup-sort");
        let mut review = DuplicateReviewState::default();

        refresh_for(&controller, &mut page, &[], &mut review);
        assert_eq!(page.pairs.len(), 3);
        assert_eq!(page.pairs[0].a, large_a);
        assert_eq!(page.pairs[1].a, medium_a);
        assert_eq!(page.pairs[2].a, small_a);

        page.search_context.sort_type = "min_filesize".to_string();
        page.search_context.sort_ascending = true;
        refresh_for(&controller, &mut page, &[], &mut review);
        assert_eq!(page.pairs[0].a, small_a);
        assert_eq!(page.pairs[2].a, large_a);

        page.search_context.sort_type = "max_filesize".to_string();
        page.search_context.sort_ascending = true;
        refresh_for(&controller, &mut page, &[], &mut review);
        assert_eq!(page.pairs[0].a, small_a);

        // An unknown sort from an older session falls back to max filesize.
        page.search_context.sort_type = "nonsense".to_string();
        page.search_context.sort_ascending = false;
        refresh_for(&controller, &mut page, &[], &mut review);
        assert_eq!(page.search_context.sort_type, "max_filesize");
        assert_eq!(page.pairs[0].a, large_a);

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn better_merge_follows_content_merge_options() {
        let directory = temp_directory("merge-options");
        let controller = test_controller(&directory);
        let (keep, discard, keep_id, discard_id) = add_sized_pair(&controller, 91, 1_000, 1_000, 2);
        let db = controller.db().db();

        add_tag(&controller, &keep, "keep tag");
        add_tag(&controller, &discard, "moved tag");
        url_map::add_url(db, keep_id, "https://keep.example").unwrap();
        url_map::add_url(db, discard_id, "https://move.example").unwrap();
        notes::set_note(db, keep_id, "note", "keep note").unwrap();
        notes::set_note(db, discard_id, "note", "discard note").unwrap();
        notes::set_note(db, discard_id, "other", "other note").unwrap();
        files_timestamps::set_file_timestamp(db, keep_id, TimestampType::ModifiedFile, Some(2_000))
            .unwrap();
        files_timestamps::set_file_timestamp(
            db,
            discard_id,
            TimestampType::ModifiedFile,
            Some(1_000),
        )
        .unwrap();

        let favourites = controller
            .services()
            .get_service(b"favourites")
            .unwrap()
            .service_id;
        ratings::set_rating(db, favourites, discard_id, crate::metadata::Rating::Like).unwrap();

        let mut options = DuplicateContentMergeOptions::better_default();
        options.sync_archive_action = ArchiveSyncAction::IfOneDoBoth;

        merge_pair(&controller, &keep, &discard, true, &options).unwrap();

        let tag_service = query::tag_service_id(&controller).unwrap();
        let keep_tags =
            mappings::get_tags(db, tag_service, ContentStatus::Current, keep_id).unwrap();
        let discard_tags =
            mappings::get_tags(db, tag_service, ContentStatus::Current, discard_id).unwrap();

        assert!(keep_tags.contains(&"moved tag".to_string()));
        assert!(keep_tags.contains(&"keep tag".to_string()));
        assert!(discard_tags.is_empty());

        let keep_urls = url_map::get_urls(db, keep_id).unwrap();
        assert!(keep_urls.contains(&"https://move.example".to_string()));

        let keep_notes = notes::get_notes(db, keep_id).unwrap();
        assert_eq!(
            keep_notes.get("note").map(String::as_str),
            Some("keep note")
        );
        assert_eq!(
            keep_notes.get("other").map(String::as_str),
            Some("other note")
        );

        assert_eq!(
            files_timestamps::get_file_timestamp(db, keep_id, TimestampType::ModifiedFile).unwrap(),
            Some(1_000)
        );
        assert!(
            ratings::get_rating(db, favourites, keep_id)
                .unwrap()
                .is_some()
        );
        assert!(
            ratings::get_rating(db, favourites, discard_id)
                .unwrap()
                .is_none()
        );
        assert!(!files_storage::is_inbox(db, keep_id).unwrap());
        assert!(!files_storage::is_inbox(db, discard_id).unwrap());

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn same_quality_merge_copies_in_both_directions() {
        let directory = temp_directory("merge-same");
        let controller = test_controller(&directory);
        let (keep, discard, _keep_id, discard_id) =
            add_sized_pair(&controller, 101, 1_000, 1_000, 1);
        let db = controller.db().db();

        add_tag(&controller, &keep, "keep only");
        add_tag(&controller, &discard, "discard only");
        url_map::add_url(db, discard_id, "https://discard.example").unwrap();

        let options = DuplicateContentMergeOptions::same_default();

        merge_pair(&controller, &keep, &discard, false, &options).unwrap();

        let tag_service = query::tag_service_id(&controller).unwrap();
        let keep_tags = mappings::get_tags(
            db,
            tag_service,
            ContentStatus::Current,
            query::hash_id_for(&controller, &keep).unwrap(),
        )
        .unwrap();
        let discard_tags =
            mappings::get_tags(db, tag_service, ContentStatus::Current, discard_id).unwrap();

        assert!(keep_tags.contains(&"discard only".to_string()));
        assert!(discard_tags.contains(&"keep only".to_string()));
        assert_eq!(
            url_map::get_urls(db, query::hash_id_for(&controller, &keep).unwrap()).unwrap(),
            vec!["https://discard.example".to_string()]
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn saved_merge_options_are_loaded_per_action() {
        let directory = temp_directory("merge-save");
        let controller = test_controller(&directory);

        let mut options = DuplicateContentMergeOptions::better_default();
        options.set_tag_service_action("local tags", MergeAction::TwoWayMerge);
        options.sync_notes_action = MergeAction::None;

        save_merge_options(&controller, MergeActionKind::Better, options.clone());

        let loaded = merge_options(&controller, MergeActionKind::Better);

        assert_eq!(loaded, options);
        assert_eq!(loaded.tag_action("local tags"), MergeAction::TwoWayMerge);

        let default_same = merge_options(&controller, MergeActionKind::Same);
        assert_eq!(default_same.sync_urls_action, MergeAction::TwoWayMerge);

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn merge_options_action_saves_without_an_open_duplicates_page() {
        let directory = temp_directory("merge-action");
        let controller = test_controller(&directory);
        let mut state = AppState::new(std::sync::Arc::new(controller));

        let payload = serde_json::json!({
            "action": "same",
            "options": {
                "sync_archive_action": "always_both",
                "sync_file_modified_date_action": "none",
                "sync_notes_action": "two_way_merge",
                "sync_urls_action": "copy",
                "tag_service_actions": { "local tags": "copy" },
                "rating_service_actions": { "favourites": "two_way_merge" }
            }
        })
        .to_string();

        let commands = save_merge_options_action(&mut state, &payload);

        assert!(matches!(
            commands.first(),
            Some(UiCommand::NotifyMessage(_))
        ));

        let saved = merge_options(&state.controller, MergeActionKind::Same);
        assert_eq!(saved.sync_archive_action, ArchiveSyncAction::AlwaysBoth);
        assert_eq!(saved.sync_file_modified_date_action, MergeAction::None);
        assert_eq!(saved.tag_action("local tags"), MergeAction::Copy);
        assert_eq!(saved.rating_action("favourites"), MergeAction::TwoWayMerge);

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn custom_action_dialog_exposes_pair_and_options() {
        let directory = temp_directory("custom-dialog");
        let mut state = action_state(&directory);
        let controller = state.controller.clone();
        add_sized_pair(&controller, 101, 1_000, 2_000, 1);
        action(&mut state, "dup-actions", "duplicates_refresh", "");

        let page = state.find_duplicates("dup-actions").unwrap();
        let value: Value =
            serde_json::from_str(&custom_action_dialog_json(&controller, page)).unwrap();

        assert_eq!(value["page_id"], "dup-actions");
        assert!(value["pair"]["a"]["hash"].is_string());
        assert!(value["pair"]["b"]["thumb"].is_string());
        assert_eq!(value["duplicate_types"].as_array().unwrap().len(), 4);
        assert_eq!(value["delete_modes"].as_array().unwrap().len(), 4);
        assert!(value["better"].is_object());
        assert!(value["tag_services"].is_array());

        let _ = std::fs::remove_dir_all(directory);
    }

    fn action_state(directory: &std::path::Path) -> AppState {
        let controller = test_controller(directory);
        let mut state = AppState::new(std::sync::Arc::new(controller));
        state.root.add(crate::gui::app::model::PageNode::Duplicates(
            DuplicatesPage::new("dup-actions"),
        ));
        state.active = Some("dup-actions".to_string());

        state
    }

    fn add_note(controller: &Controller, hash: &[u8], name: &str, text: &str) {
        let hash_id = query::hash_id_for(controller, hash).unwrap();
        notes::set_note(controller.db().db(), hash_id, name, text).unwrap();
    }

    #[test]
    fn keep_both_and_alternates_do_not_trash_files() {
        let directory = temp_directory("keep-both");
        let mut state = action_state(&directory);
        let controller = state.controller.clone();
        let (_, _, first_id, second_id) = add_sized_pair(&controller, 61, 1_000, 4_000, 2);
        action(&mut state, "dup-actions", "duplicates_refresh", "");
        let pair = state
            .find_duplicates("dup-actions")
            .unwrap()
            .current()
            .cloned()
            .unwrap();

        let commands = action(&mut state, "dup-actions", "duplicates_keep_a_keep", "");
        assert!(matches!(
            commands.first(),
            Some(UiCommand::UpdateDuplicates(_, _))
        ));

        let db = controller.db().db();
        let my_files = query::local_file_service_id(&controller, "my files").unwrap();
        let trash = query::trash_service_id(&controller).unwrap();
        let in_my_files =
            files_storage::get_hash_ids(db, my_files, ContentStatus::Current).unwrap();

        assert!(in_my_files.contains(&first_id));
        assert!(in_my_files.contains(&second_id));
        assert!(
            files_storage::get_hash_ids(db, trash, ContentStatus::Current)
                .unwrap()
                .is_empty()
        );
        assert!(
            matches!(
                duplicates::get_alternate_group_ids_for_hash(db, first_id)
                    .unwrap()
                    .as_slice(),
                [_]
            ),
            "a keep-both decision should create one alternates group"
        );
        assert!(
            duplicates::get_candidate_pairs(db, None)
                .unwrap()
                .is_empty(),
            "the decided potential pair should be gone"
        );
        assert!(pair.a != pair.b);

        // A second pair can be marked as related alternates without deleting.
        let (_, _, third_id, fourth_id) = add_sized_pair(&controller, 63, 1_000, 2_000, 1);
        action(&mut state, "dup-actions", "duplicates_refresh", "");
        action(&mut state, "dup-actions", "duplicates_alternates", "");

        assert_eq!(
            duplicates::get_alternate_group_ids_for_hash(db, third_id)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            duplicates::get_alternate_group_ids_for_hash(db, fourth_id)
                .unwrap()
                .len(),
            1
        );
        assert!(
            files_storage::get_hash_ids(db, trash, ContentStatus::Current)
                .unwrap()
                .is_empty()
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn custom_action_uses_per_decision_merge_options_and_delete_choice() {
        let directory = temp_directory("custom");
        let mut state = action_state(&directory);
        let controller = state.controller.clone();
        let (_, _, _, _) = add_sized_pair(&controller, 71, 1_000, 4_000, 2);
        action(&mut state, "dup-actions", "duplicates_refresh", "");

        let pair = state
            .find_duplicates("dup-actions")
            .unwrap()
            .current()
            .cloned()
            .unwrap();
        let (a_id, b_id) = (
            query::hash_id_for(&controller, &pair.a).unwrap(),
            query::hash_id_for(&controller, &pair.b).unwrap(),
        );

        add_note(&controller, &pair.a, "note", "from A");

        let mut better = DuplicateContentMergeOptions::better_default();
        better.sync_notes_action = MergeAction::Copy;
        let payload = serde_json::json!({
            "action": "better",
            "better_side": "b",
            "delete": "a",
            "better": serde_json::to_value(better).unwrap(),
            "same": serde_json::to_value(DuplicateContentMergeOptions::same_default()).unwrap(),
        })
        .to_string();

        action(&mut state, "dup-actions", "duplicates_custom", &payload);

        let db = controller.db().db();
        let my_files = query::local_file_service_id(&controller, "my files").unwrap();
        let trash = query::trash_service_id(&controller).unwrap();
        let in_my_files =
            files_storage::get_hash_ids(db, my_files, ContentStatus::Current).unwrap();
        let in_trash = files_storage::get_hash_ids(db, trash, ContentStatus::Current).unwrap();

        assert!(in_trash.contains(&a_id), "delete=A should trash A");
        assert!(in_my_files.contains(&b_id), "B should be kept");
        assert_eq!(
            notes::get_note(db, b_id, "note").unwrap().as_deref(),
            Some("from A"),
            "the custom better options copied A's note to B"
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn group_actions_clear_groups_false_positives_and_potentials() {
        let directory = temp_directory("group-actions");
        let mut state = action_state(&directory);
        let controller = state.controller.clone();
        let (first_hash, _second_hash, first_id, second_id) =
            add_sized_pair(&controller, 81, 1_000, 2_000, 2);
        action(&mut state, "dup-actions", "duplicates_refresh", "");
        let pair = state
            .find_duplicates("dup-actions")
            .unwrap()
            .current()
            .cloned()
            .unwrap();
        let db = controller.db().db();

        duplicates::create_alternate_group(db, &[first_id, second_id]).unwrap();
        duplicates::add_false_positive(db, first_id.min(second_id), first_id.max(second_id))
            .unwrap();

        action(
            &mut state,
            "dup-actions",
            "duplicates_group_action",
            r#"{"kind":"clear_false_positives","target":"both"}"#,
        );

        assert!(
            !duplicates::is_false_positive(db, first_id.min(second_id), first_id.max(second_id))
                .unwrap()
        );

        action(
            &mut state,
            "dup-actions",
            "duplicates_group_action",
            r#"{"kind":"remove_from_alternate_group","target":"a"}"#,
        );

        let remaining = if pair.a == first_hash {
            second_id
        } else {
            first_id
        };
        assert!(
            duplicates::get_alternate_group_ids_for_hash(db, remaining)
                .unwrap()
                .len()
                <= 1
        );

        // A reset search rebuilds a pair from the stored perceptual hashes.
        duplicates::remove_potential_pair(db, first_id.min(second_id), first_id.max(second_id))
            .unwrap();
        similar_files::set_perceptual_hashes(db, first_id, &[vec![1, 2, 3, 4, 5, 6, 7, 8]])
            .unwrap();
        similar_files::set_perceptual_hashes(db, second_id, &[vec![1, 2, 3, 4, 5, 6, 7, 9]])
            .unwrap();
        action(
            &mut state,
            "dup-actions",
            "duplicates_group_action",
            r#"{"kind":"reset_potential_search","target":"both"}"#,
        );

        assert!(
            !duplicates::get_candidate_pairs(db, None)
                .unwrap()
                .is_empty(),
            "reset potential search should queue the pair again"
        );

        action(
            &mut state,
            "dup-actions",
            "duplicates_group_action",
            r#"{"kind":"remove_potentials","target":"both"}"#,
        );

        assert!(
            duplicates::get_candidate_pairs(db, None)
                .unwrap()
                .is_empty()
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn duplicate_group_membership_can_be_removed_dissolved() {
        let directory = temp_directory("duplicate-groups");
        let mut state = action_state(&directory);
        let controller = state.controller.clone();
        let (_, _, first_id, second_id) = add_sized_pair(&controller, 91, 1_000, 2_000, 1);
        action(&mut state, "dup-actions", "duplicates_refresh", "");
        let db = controller.db().db();

        duplicates::set_duplicate_group(db, 10, first_id, &[first_id, second_id]).unwrap();
        assert_eq!(
            duplicates::get_duplicate_group_members(db, 10).unwrap(),
            vec![first_id, second_id]
        );

        action(
            &mut state,
            "dup-actions",
            "duplicates_group_action",
            r#"{"kind":"dissolve_duplicate_group","target":"both"}"#,
        );

        assert!(
            duplicates::get_duplicate_group_members(db, 10)
                .unwrap()
                .is_empty()
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn pair_thumbnail_hashes_flatten_and_dedupe_pairs() {
        let mut page = DuplicatesPage::new("dup-media");
        page.pairs.push(DuplicatePairView {
            a: vec![1, 2],
            b: vec![3, 4],
            distance: 1,
        });
        page.pairs.push(DuplicatePairView {
            a: vec![3, 4],
            b: vec![5, 6],
            distance: 2,
        });

        assert_eq!(
            pair_thumbnail_hashes(&page),
            vec![vec![1, 2], vec![3, 4], vec![5, 6]]
        );
    }

    #[test]
    fn pair_thumbnail_hashes_empty_for_empty_queue() {
        let page = DuplicatesPage::new("dup-media-empty");
        assert!(pair_thumbnail_hashes(&page).is_empty());
    }

    #[test]
    fn duplicates_media_count_status_strings() {
        assert_eq!(duplicates_media_count_status(0), "no potential duplicate pairs");
        assert_eq!(duplicates_media_count_status(1), "1 potential duplicate pair");
        assert_eq!(duplicates_media_count_status(3), "3 potential duplicate pairs");
    }

    #[test]
    fn duplicates_media_empty_status_string() {
        assert_eq!(
            duplicates_media_empty_status(),
            "the potential duplicate queue is empty"
        );
    }

    #[test]
    fn split_into_batches_splits_evenly() {
        let items: Vec<i32> = (0..7).collect();
        let batches = split_into_batches(&items, 3);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0], vec![0, 1, 2]);
        assert_eq!(batches[1], vec![3, 4, 5]);
        assert_eq!(batches[2], vec![6]);
    }

    #[test]
    fn split_into_batches_empty_input() {
        let items: Vec<i32> = Vec::new();
        assert!(split_into_batches(&items, 3).is_empty());
    }

    #[test]
    fn split_into_batches_zero_size_means_one_batch() {
        let items: Vec<i32> = (0..4).collect();
        let batches = split_into_batches(&items, 0);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 4);
    }

    #[test]
    fn preparation_progress_basic() {
        assert_eq!(preparation_progress(0, 10), 0.0);
        assert_eq!(preparation_progress(5, 10), 0.5);
        assert_eq!(preparation_progress(10, 10), 1.0);
        assert_eq!(preparation_progress(5, 0), 0.0);
        assert_eq!(preparation_progress(15, 10), 1.0);
    }

    #[test]
    fn preparation_progress_percent_caps_at_99_9() {
        assert_eq!(preparation_progress_percent(0, 10), "0.0%");
        assert_eq!(preparation_progress_percent(5, 10), "50.0%");
        assert_eq!(preparation_progress_percent(10, 10), "99.9%");
        assert_eq!(preparation_progress_percent(3, 7), "42.9%");
    }

    #[test]
    fn preparation_search_status_strings() {
        assert_eq!(
            preparation_search_status(0, 10),
            "Have not yet searched at this distance."
        );
        assert_eq!(
            preparation_search_status(5, 10),
            "Searched 5 of 10 files at this distance."
        );
        assert_eq!(
            preparation_search_status(10, 10),
            "All potential duplicates found at this distance."
        );
        assert_eq!(
            preparation_search_status(0, 0),
            "All potential duplicates found at this distance."
        );
    }

    #[test]
    fn preparation_tab_name_adds_suffix() {
        assert_eq!(preparation_tab_name(0, 0, false), "preparation");
        assert_eq!(preparation_tab_name(10, 10, false), "preparation");
        assert_eq!(
            preparation_tab_name(5, 10, false),
            "preparation (50.0% done)"
        );
    }

    #[test]
    fn preparation_tab_name_hides_when_caught_up() {
        assert_eq!(preparation_tab_name(100, 101, true), "preparation");
        assert_eq!(
            preparation_tab_name(100, 101, false),
            "preparation (99.0% done)"
        );
        assert_eq!(
            preparation_tab_name(99, 100, false),
            "preparation (99.0% done)"
        );
    }

    #[test]
    fn should_auto_commit_respects_threshold() {
        assert!(!should_auto_commit(1, None));
        assert!(should_auto_commit(1, Some(1)));
        assert!(should_auto_commit(2, Some(5)));
        assert!(!should_auto_commit(6, Some(5)));
        assert!(!should_auto_commit(1, Some(0)));
        assert!(!should_auto_commit(1, Some(-1)));
    }

    #[test]
    fn preparation_batch_status_strings() {
        assert_eq!(preparation_batch_status(0, 0, 5), "no potential duplicates");
        assert_eq!(preparation_batch_status(5, 0, 5), "preparing");
        assert_eq!(preparation_batch_status(5, 5, 5), "prepared");
        assert_eq!(
            preparation_batch_status(6, 5, 5),
            "6 potential duplicate pairs (batch of 5, max 5)"
        );
    }
}
