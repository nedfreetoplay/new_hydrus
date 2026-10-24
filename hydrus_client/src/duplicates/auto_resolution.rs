//! Duplicates auto-resolution from `ClientDuplicatesAutoResolution.py`.

use crate::duplicates::DuplicatePairDecision;
use crate::media::MediaResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateComparator {
    Always,
    HigherResolution,
    LargerFile,
    MoreTags,
    Older,
    FewerTags,
}

impl DuplicateComparator {
    fn first_is_better(&self, a: &MediaResult, b: &MediaResult) -> bool {
        match self {
            Self::Always => true,
            Self::HigherResolution => resolution_score(a) > resolution_score(b),
            Self::LargerFile => a.get_size() > b.get_size(),
            Self::MoreTags => tag_count(a) > tag_count(b),
            Self::FewerTags => tag_count(a) < tag_count(b),
            Self::Older => {
                let a_time = a
                    .times_manager
                    .times
                    .values()
                    .flatten()
                    .copied()
                    .min()
                    .unwrap_or(i64::MAX);
                let b_time = b
                    .times_manager
                    .times
                    .values()
                    .flatten()
                    .copied()
                    .min()
                    .unwrap_or(i64::MAX);
                a_time < b_time
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DuplicatesAutoResolutionRule {
    pub name: String,
    pub comparator: DuplicateComparator,
    pub action: DuplicatePairDecision,
}

impl DuplicatesAutoResolutionRule {
    pub fn new(
        name: impl Into<String>,
        comparator: DuplicateComparator,
        action: DuplicatePairDecision,
    ) -> Self {
        Self {
            name: name.into(),
            comparator,
            action,
        }
    }

    /// Returns the decision to apply, or `None` when the rule does not match.
    pub fn evaluate(&self, a: &MediaResult, b: &MediaResult) -> Option<DuplicatePairDecision> {
        if self.comparator.first_is_better(a, b) {
            Some(self.action.clone())
        } else if self.comparator.first_is_better(b, a) {
            Some(invert_merge_sides(&self.action))
        } else {
            None
        }
    }
}

#[derive(Debug, Default)]
pub struct DuplicatesAutoResolutionManager {
    rules: Vec<DuplicatesAutoResolutionRule>,
}

impl DuplicatesAutoResolutionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_rule(&mut self, rule: DuplicatesAutoResolutionRule) {
        self.rules.push(rule);
    }

    pub fn remove_rule(&mut self, name: &str) -> bool {
        let previous = self.rules.len();
        self.rules.retain(|rule| rule.name != name);
        previous != self.rules.len()
    }

    pub fn get_rules(&self) -> &[DuplicatesAutoResolutionRule] {
        &self.rules
    }

    pub fn evaluate(&self, a: &MediaResult, b: &MediaResult) -> Option<DuplicatePairDecision> {
        self.rules.iter().find_map(|rule| rule.evaluate(a, b))
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

fn invert_merge_sides(decision: &DuplicatePairDecision) -> DuplicatePairDecision {
    match decision {
        DuplicatePairDecision::Delete { delete_a, delete_b } => DuplicatePairDecision::Delete {
            delete_a: *delete_b,
            delete_b: *delete_a,
        },
        other => other.clone(),
    }
}

fn resolution_score(media: &MediaResult) -> u64 {
    media
        .get_resolution()
        .map(|(width, height)| width as u64 * height as u64)
        .unwrap_or(0)
}

fn tag_count(media: &MediaResult) -> usize {
    media
        .tags_manager
        .tags
        .get_service_keys()
        .into_iter()
        .map(|service_key| media.tags_manager.get_tags(service_key).len())
        .sum()
}

#[cfg(test)]
mod tests;
