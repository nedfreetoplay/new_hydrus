use super::*;

#[test]
fn default_merge_options_copy_everything() {
    let options = DuplicateContentMergeOptions::default();

    assert_eq!(options.sync_archive_action, ArchiveSyncAction::IfOneDoBoth);
    assert_eq!(options.sync_notes_action, MergeAction::Copy);
    assert!(options.get_merge_summary().contains("archive"));
}

#[test]
fn better_defaults_move_local_tags_and_favourites() {
    let options = DuplicateContentMergeOptions::better_default();

    assert_eq!(options.tag_action("local tags"), MergeAction::Move);
    assert_eq!(options.rating_action("favourites"), MergeAction::Move);
    assert_eq!(options.sync_archive_action, ArchiveSyncAction::AlwaysBoth);
    assert_eq!(options.sync_notes_action, MergeAction::Copy);
}

#[test]
fn same_defaults_merge_in_both_directions() {
    let options = DuplicateContentMergeOptions::same_default();

    assert_eq!(options.tag_action("local tags"), MergeAction::TwoWayMerge);
    assert_eq!(
        options.sync_file_modified_date_action,
        MergeAction::TwoWayMerge
    );
    assert_eq!(options.sync_urls_action, MergeAction::TwoWayMerge);
}

#[test]
fn unlisted_services_are_left_untouched() {
    let options = DuplicateContentMergeOptions::default();

    assert_eq!(options.tag_action("my tags"), MergeAction::None);
    assert_eq!(options.rating_action("my rating"), MergeAction::None);
}

#[test]
fn merge_options_store_per_service_actions() {
    let mut options = DuplicateContentMergeOptions::new();
    options.set_tag_service_action("local tags", MergeAction::TwoWayMerge);

    assert_eq!(
        options.tag_service_actions["local tags"],
        MergeAction::TwoWayMerge
    );
}

#[test]
fn merge_options_serialise_with_service_names() {
    let mut options = DuplicateContentMergeOptions::better_default();
    options.set_rating_service_action("favourites", MergeAction::Copy);

    let json = serde_json::to_value(&options).unwrap();
    let restored: DuplicateContentMergeOptions = serde_json::from_value(json).unwrap();

    assert_eq!(restored, options);
    assert_eq!(restored.tag_action("local tags"), MergeAction::Move);
    assert_eq!(restored.rating_action("favourites"), MergeAction::Copy);
}

#[test]
fn decisions_have_terminal_semantics() {
    assert!(DuplicatePairDecision::Skip.is_skip());
    assert!(!DuplicatePairDecision::SkipManual.is_terminal());
    assert!(DuplicatePairDecision::Approve.is_terminal());
}

#[test]
fn queue_yields_pairs_in_order() {
    let mut queue = PotentialDuplicatesPairQueue::new();
    queue.add_pair(PotentialDuplicatePair::new(vec![1], vec![2], 4));
    queue.add_pair(PotentialDuplicatePair::new(vec![3], vec![4], 8));

    let first = queue.get_next().unwrap();

    assert_eq!(first.hash_a, vec![1]);
    assert_eq!(queue.len(), 1);
}

#[test]
fn decided_pairs_can_be_removed() {
    let mut queue = PotentialDuplicatesPairQueue::new();
    queue.add_pair(PotentialDuplicatePair::new(vec![1], vec![2], 4));

    assert!(queue.decide_next(DuplicatePairDecision::Skip));
    assert_eq!(queue.remove_decided(), 1);
    assert!(queue.is_empty());
}
