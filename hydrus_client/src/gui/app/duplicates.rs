//! Duplicate processing page logic.
//!
//! The expensive shape search already happens during import; this module turns
//! the stored potential pairs into a filter queue and applies the user's merge,
//! false-positive and skip decisions.

use hydrus_core::hydrus_constants::ContentStatus;

use crate::controller::Controller;
use crate::db::{duplicates, files_storage, mappings, master, master_tags, url_map};
use crate::gui::app::model::{DuplicatePairView, DuplicatesPage};
use crate::gui::app::{AppState, UiCommand, query, render};

pub fn refresh(state: &mut AppState, page_id: &str) -> Vec<UiCommand> {
    let controller = state.controller.clone();
    let rules = state.auto_rules.clone();

    let Some(page) = state.find_duplicates_mut(page_id) else {
        return vec![UiCommand::NotifyMessage(
            "no duplicates page selected".to_string(),
        )];
    };

    load_queue(&controller, page, &rules);
    duplicates_commands(state, page_id)
}

/// Fills a freshly created page without needing the page tree.
pub fn refresh_for(
    controller: &Controller,
    page: &mut DuplicatesPage,
    rules: &[super::auto_resolution::AutoResolutionRule],
) {
    load_queue(controller, page, rules);
}

pub fn action(state: &mut AppState, page_id: &str, action: &str, payload: &str) -> Vec<UiCommand> {
    let controller = state.controller.clone();
    let rules = state.auto_rules.clone();

    let Some(page) = state.find_duplicates_mut(page_id) else {
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
        "duplicates_refresh" => {
            load_queue(&controller, page, &rules);
        }
        "duplicates_keep_a" | "duplicates_keep_b" | "duplicates_same" => {
            let Some(pair) = page.current().cloned() else {
                return vec![UiCommand::NotifyMessage(
                    "no duplicate pair left".to_string(),
                )];
            };

            let (keep, discard, trash) = match action {
                "duplicates_keep_a" => (pair.a.clone(), pair.b.clone(), true),
                "duplicates_keep_b" => (pair.b.clone(), pair.a.clone(), true),
                _ => (pair.a.clone(), pair.b.clone(), false),
            };

            match merge_pair(&controller, &keep, &discard, trash) {
                Ok(()) => {
                    page.remove_current();
                    page.status = if trash {
                        format!("kept {}; moved the other file to trash", query::hex(&keep))
                    } else {
                        "files marked as duplicates; both kept".to_string()
                    };
                }
                Err(error) => {
                    page.status = format!("merge failed: {error}");
                }
            }
        }
        "duplicates_false" => {
            let Some(pair) = page.current().cloned() else {
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
) {
    let max_distance = controller
        .options()
        .get_integer("similar_files_duplicate_pairs_search_distance");
    let max_distance = (max_distance > 0).then_some(max_distance);

    match duplicates::get_candidate_pairs(controller.db().db(), max_distance) {
        Ok(pairs) => {
            page.searched = pairs.len() as i64;
            page.pairs = pairs
                .into_iter()
                .filter_map(|(smaller_id, larger_id, distance)| {
                    let a = master::get_hash(controller.db().db(), smaller_id)
                        .ok()
                        .flatten()?;
                    let b = master::get_hash(controller.db().db(), larger_id)
                        .ok()
                        .flatten()?;

                    Some(DuplicatePairView { a, b, distance })
                })
                .collect();
            page.index = 0;

            let applied = if controller
                .options()
                .get_boolean("duplicates_auto_resolution_enabled")
            {
                apply_rules(controller, page, rules)
            } else {
                0
            };

            page.status = match page.pairs.len() {
                0 => "no potential duplicates".to_string(),
                1 => "1 potential duplicate pair".to_string(),
                count => format!("{count} potential duplicate pairs"),
            };

            if applied > 0 {
                page.status = format!("{}; {applied} auto-resolved", page.status);
            }
        }
        Err(error) => {
            page.pairs.clear();
            page.index = 0;
            page.status = format!("could not load the duplicate queue: {error}");
        }
    }
}

fn apply_rules(
    controller: &Controller,
    page: &mut DuplicatesPage,
    rules: &[super::auto_resolution::AutoResolutionRule],
) -> usize {
    if rules.is_empty() {
        return 0;
    }

    use super::auto_resolution::AutoDecision;

    let mut kept = Vec::new();
    let mut applied = 0;

    for pair in std::mem::take(&mut page.pairs) {
        let decision = rules
            .iter()
            .find_map(|rule| rule.evaluate(controller, &pair.a, &pair.b));

        let result = match decision {
            Some(AutoDecision::KeepFirst) => merge_pair(controller, &pair.a, &pair.b, true),
            Some(AutoDecision::KeepSecond) => merge_pair(controller, &pair.b, &pair.a, true),
            Some(AutoDecision::Same) => merge_pair(controller, &pair.a, &pair.b, false),
            Some(AutoDecision::NotDuplicates) => mark_false_positive(controller, &pair.a, &pair.b),
            Some(AutoDecision::DeleteBoth) => delete_both(controller, &pair.a, &pair.b),
            None => {
                kept.push(pair);
                continue;
            }
        };

        match result {
            Ok(()) => applied += 1,
            Err(error) => {
                page.status = format!("auto-resolution failed: {error}");
                kept.push(pair);
            }
        }
    }

    page.pairs = kept;
    page.index = 0;

    applied
}

fn merge_pair(
    controller: &Controller,
    keep: &[u8],
    discard: &[u8],
    trash_discard: bool,
) -> Result<(), String> {
    let db = controller.db().db();

    let keep_id = query::hash_id_for(controller, keep)
        .ok_or_else(|| "kept file is not in the database".to_string())?;
    let discard_id = query::hash_id_for(controller, discard)
        .ok_or_else(|| "other file is not in the database".to_string())?;

    let tag_service_id =
        query::tag_service_id(controller).ok_or_else(|| "no local tag service".to_string())?;

    let keep_tags =
        mappings::get_tags(db, tag_service_id, ContentStatus::Current, keep_id).unwrap_or_default();
    let discard_tags = mappings::get_tags(db, tag_service_id, ContentStatus::Current, discard_id)
        .unwrap_or_default();

    for tag in &discard_tags {
        if !keep_tags.contains(tag) {
            if let Ok(tag_id) = master_tags::get_tag_id(db, tag) {
                let _ = mappings::add_mappings(
                    db,
                    tag_service_id,
                    ContentStatus::Current,
                    keep_id,
                    &[tag_id],
                );
            }
        }
    }

    for url in url_map::get_urls(db, discard_id).unwrap_or_default() {
        let _ = url_map::add_url(db, keep_id, &url);
    }

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

fn duplicates_commands(state: &AppState, page_id: &str) -> Vec<UiCommand> {
    let Some(page) = state.find_duplicates(page_id) else {
        return vec![];
    };

    vec![
        UiCommand::UpdateDuplicates(
            page_id.to_string(),
            render::duplicates_json(&state.controller, page),
        ),
        UiCommand::Status(render::status_json(state)),
    ]
}
