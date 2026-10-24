//! Duplicate merge options and pair decisions (`ClientDuplicates.py`,
//! `ClientPotentialDuplicatesPairFactory.py`).

use std::collections::BTreeMap;

use hydrus_core::hydrus_constants::ContentMergeAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateContentMergeOptions {
    pub sync_archive_action: ContentMergeAction,
    pub sync_file_modified_date_action: ContentMergeAction,
    pub sync_notes_action: ContentMergeAction,
    pub sync_urls_action: ContentMergeAction,
    pub tag_service_actions: BTreeMap<Vec<u8>, ContentMergeAction>,
    pub rating_service_actions: BTreeMap<Vec<u8>, ContentMergeAction>,
}

impl Default for DuplicateContentMergeOptions {
    fn default() -> Self {
        Self {
            sync_archive_action: ContentMergeAction::Copy,
            sync_file_modified_date_action: ContentMergeAction::Copy,
            sync_notes_action: ContentMergeAction::Copy,
            sync_urls_action: ContentMergeAction::Copy,
            tag_service_actions: BTreeMap::new(),
            rating_service_actions: BTreeMap::new(),
        }
    }
}

impl DuplicateContentMergeOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_tag_service_action(
        &mut self,
        service_key: impl Into<Vec<u8>>,
        action: ContentMergeAction,
    ) {
        self.tag_service_actions.insert(service_key.into(), action);
    }

    pub fn set_rating_service_action(
        &mut self,
        service_key: impl Into<Vec<u8>>,
        action: ContentMergeAction,
    ) {
        self.rating_service_actions
            .insert(service_key.into(), action);
    }

    pub fn get_merge_summary(&self) -> String {
        format!(
            "archive: {}, modified date: {}, notes: {}, urls: {}",
            self.sync_archive_action.number_merge_str(),
            self.sync_file_modified_date_action.number_merge_str(),
            self.sync_notes_action.number_merge_str(),
            self.sync_urls_action.number_merge_str()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuplicatePairDecision {
    Skip,
    SkipManual,
    SkipAuto,
    Merge(DuplicateContentMergeOptions),
    Delete { delete_a: bool, delete_b: bool },
    Approve,
    Deny,
}

impl DuplicatePairDecision {
    pub fn is_skip(&self) -> bool {
        matches!(self, Self::Skip | Self::SkipManual | Self::SkipAuto)
    }

    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::SkipManual)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PotentialDuplicatePair {
    pub hash_a: Vec<u8>,
    pub hash_b: Vec<u8>,
    pub distance: i64,
    pub decision: Option<DuplicatePairDecision>,
}

impl PotentialDuplicatePair {
    pub fn new(hash_a: Vec<u8>, hash_b: Vec<u8>, distance: i64) -> Self {
        Self {
            hash_a,
            hash_b,
            distance,
            decision: None,
        }
    }

    pub fn decide(&mut self, decision: DuplicatePairDecision) {
        self.decision = Some(decision);
    }

    pub fn is_decided(&self) -> bool {
        self.decision.is_some()
    }
}

#[derive(Debug, Default)]
pub struct PotentialDuplicatesPairQueue {
    pairs: Vec<PotentialDuplicatePair>,
}

impl PotentialDuplicatesPairQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_pair(&mut self, pair: PotentialDuplicatePair) {
        self.pairs.push(pair);
    }

    pub fn add_pairs(&mut self, pairs: impl IntoIterator<Item = PotentialDuplicatePair>) {
        self.pairs.extend(pairs);
    }

    pub fn get_next(&mut self) -> Option<PotentialDuplicatePair> {
        if self.pairs.is_empty() {
            return None;
        }

        Some(self.pairs.remove(0))
    }

    pub fn peek(&self) -> Option<&PotentialDuplicatePair> {
        self.pairs.first()
    }

    pub fn decide_next(&mut self, decision: DuplicatePairDecision) -> bool {
        match self.pairs.first_mut() {
            Some(pair) => {
                pair.decide(decision);
                true
            }
            None => false,
        }
    }

    pub fn remove_decided(&mut self) -> usize {
        let previous = self.pairs.len();
        self.pairs.retain(|pair| !pair.is_decided());
        previous - self.pairs.len()
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

#[cfg(test)]
mod tests {
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
}
