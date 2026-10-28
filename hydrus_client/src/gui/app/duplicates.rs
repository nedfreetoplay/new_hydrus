//! Duplicate processing page logic.
//!
//! The expensive shape search already happens during import; this module turns
//! the stored potential pairs into a filter queue and applies the user's merge,
//! false-positive and skip decisions.

use hydrus_core::hydrus_constants::ContentStatus;

use crate::controller::Controller;
use crate::db::{duplicates, files_storage, mappings, master, master_tags, url_map};
use crate::gui::app::auto_resolution::{self, AutoDecision, DuplicateReviewState, PreparedRule};
use crate::gui::app::model::{DuplicatePairView, DuplicatesPage};
use crate::gui::app::{query, render, AppState, UiCommand};

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
    let max_distance = controller
        .options()
        .get_integer("similar_files_duplicate_pairs_search_distance");
    let max_distance = (max_distance > 0).then_some(max_distance);
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

    match duplicates::get_candidate_pairs(controller.db().db(), max_distance) {
        Ok(pairs) => {
            let total = pairs.len();
            page.searched = total as i64;
            page.pairs.clear();
            page.index = 0;

            let mut applied = 0usize;
            let mut queued = 0usize;

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

            let batch = page.pairs.len();
            page.status = match total {
                0 => "no potential duplicates".to_string(),
                1 => "1 potential duplicate pair".to_string(),
                count => format!(
                    "{count} potential duplicate pairs (batch of {batch}, max {batch_size})"
                ),
            };

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
            page.status = format!("could not load the duplicate queue: {error}");

            (0, 0)
        }
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
        AutoDecision::KeepFirst => merge_pair(controller, a, b, true),
        AutoDecision::KeepSecond => merge_pair(controller, b, a, true),
        AutoDecision::Same => {
            merge_pair(controller, a, b, false)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{duplicates, files_metadata, files_storage};
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
        assert!(duplicates::get_candidate_pairs(controller.db().db(), None)
            .unwrap()
            .is_empty());
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
}
