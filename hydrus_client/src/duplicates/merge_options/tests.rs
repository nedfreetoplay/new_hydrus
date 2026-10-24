
use super::*;

#[test]
fn default_merge_options_copy_everything() {
    let options = DuplicateContentMergeOptions::default();

    assert_eq!(options.sync_archive_action, ContentMergeAction::Copy);
    assert!(options.get_merge_summary().contains("archive"));
}

#[test]
fn merge_options_store_per_service_actions() {
    let mut options = DuplicateContentMergeOptions::new();
    options.set_tag_service_action(b"local tags".to_vec(), ContentMergeAction::TwoWayMerge);

    assert_eq!(
        options.tag_service_actions[b"local tags".as_slice()],
        ContentMergeAction::TwoWayMerge
    );
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
