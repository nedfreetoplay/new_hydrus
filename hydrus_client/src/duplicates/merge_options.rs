//! Duplicate merge options and pair decisions (`ClientDuplicates.py`,
//! `ClientPotentialDuplicatesPairFactory.py`).
//!
//! The options decide what the merged (better, or both "same quality") files
//! receive from the worse file. Service actions are keyed by service name and
//! an unlisted service is left untouched, matching the Python default of
//! "make no change".

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeAction {
    Copy,
    Move,
    TwoWayMerge,
    None,
}

impl MergeAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Move => "move",
            Self::TwoWayMerge => "two_way_merge",
            Self::None => "none",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Copy => "copy from worse to better",
            Self::Move => "move from worse to better",
            Self::TwoWayMerge => "copy in both directions",
            Self::None => "make no change",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "move" => Self::Move,
            "two_way_merge" => Self::TwoWayMerge,
            "none" => Self::None,
            _ => Self::Copy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveSyncAction {
    None,
    #[default]
    IfOneDoBoth,
    AlwaysBoth,
}

impl ArchiveSyncAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::IfOneDoBoth => "if_one_do_both",
            Self::AlwaysBoth => "always_both",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "make no change",
            Self::IfOneDoBoth => "if one is archived, archive the other",
            Self::AlwaysBoth => "always archive both",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "none" => Self::None,
            "always_both" => Self::AlwaysBoth,
            _ => Self::IfOneDoBoth,
        }
    }
}

fn default_copy() -> MergeAction {
    MergeAction::Copy
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateContentMergeOptions {
    #[serde(default)]
    pub sync_archive_action: ArchiveSyncAction,
    #[serde(default = "default_copy")]
    pub sync_file_modified_date_action: MergeAction,
    #[serde(default = "default_copy")]
    pub sync_notes_action: MergeAction,
    #[serde(default = "default_copy")]
    pub sync_urls_action: MergeAction,
    #[serde(default)]
    pub tag_service_actions: BTreeMap<String, MergeAction>,
    #[serde(default)]
    pub rating_service_actions: BTreeMap<String, MergeAction>,
}

impl Default for DuplicateContentMergeOptions {
    fn default() -> Self {
        Self {
            sync_archive_action: ArchiveSyncAction::IfOneDoBoth,
            sync_file_modified_date_action: MergeAction::Copy,
            sync_notes_action: MergeAction::Copy,
            sync_urls_action: MergeAction::Copy,
            tag_service_actions: BTreeMap::new(),
            rating_service_actions: BTreeMap::new(),
        }
    }
}

impl DuplicateContentMergeOptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// The defaults the Python client seeds for "this is better".
    pub fn better_default() -> Self {
        let mut options = Self {
            sync_archive_action: ArchiveSyncAction::AlwaysBoth,
            sync_file_modified_date_action: MergeAction::Copy,
            sync_notes_action: MergeAction::Copy,
            sync_urls_action: MergeAction::Copy,
            tag_service_actions: BTreeMap::new(),
            rating_service_actions: BTreeMap::new(),
        };

        options.set_tag_service_action("local tags", MergeAction::Move);
        options.set_rating_service_action("favourites", MergeAction::Move);

        options
    }

    /// The defaults the Python client seeds for "same quality".
    pub fn same_default() -> Self {
        let mut options = Self {
            sync_archive_action: ArchiveSyncAction::AlwaysBoth,
            sync_file_modified_date_action: MergeAction::TwoWayMerge,
            sync_notes_action: MergeAction::TwoWayMerge,
            sync_urls_action: MergeAction::TwoWayMerge,
            tag_service_actions: BTreeMap::new(),
            rating_service_actions: BTreeMap::new(),
        };

        options.set_tag_service_action("local tags", MergeAction::TwoWayMerge);
        options.set_rating_service_action("favourites", MergeAction::TwoWayMerge);

        options
    }

    pub fn set_tag_service_action(&mut self, service_name: impl Into<String>, action: MergeAction) {
        self.tag_service_actions.insert(service_name.into(), action);
    }

    pub fn set_rating_service_action(
        &mut self,
        service_name: impl Into<String>,
        action: MergeAction,
    ) {
        self.rating_service_actions
            .insert(service_name.into(), action);
    }

    pub fn tag_action(&self, service_name: &str) -> MergeAction {
        self.tag_service_actions
            .get(service_name)
            .copied()
            .unwrap_or(MergeAction::None)
    }

    pub fn rating_action(&self, service_name: &str) -> MergeAction {
        self.rating_service_actions
            .get(service_name)
            .copied()
            .unwrap_or(MergeAction::None)
    }

    pub fn get_merge_summary(&self) -> String {
        format!(
            "archive: {}, modified date: {}, notes: {}, urls: {}",
            self.sync_archive_action.label(),
            self.sync_file_modified_date_action.label(),
            self.sync_notes_action.label(),
            self.sync_urls_action.label()
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
mod tests;
