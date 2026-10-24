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
        self.rules
            .iter()
            .find_map(|rule| rule.evaluate(a, b))
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
mod tests {
    use super::*;
    use crate::metadata::TagStatus;
    use hydrus_core::files::hydrus_file_handling::FileInfo;
    use hydrus_core::hydrus_constants::Mime;

    fn media(size: u64, resolution: (u32, u32)) -> MediaResult {
        MediaResult::new(
            vec![size as u8],
            FileInfo {
                size,
                mime: Mime::Png,
                width: Some(resolution.0),
                height: Some(resolution.1),
                duration_ms: None,
                num_frames: None,
                has_audio: false,
                num_words: None,
            },
        )
    }

    #[test]
    fn larger_file_rule_matches_and_deletes_worse() {
        let rule = DuplicatesAutoResolutionRule::new(
            "big",
            DuplicateComparator::LargerFile,
            DuplicatePairDecision::Delete {
                delete_a: false,
                delete_b: true,
            },
        );
        let a = media(100, (10, 10));
        let b = media(200, (10, 10));

        assert_eq!(
            rule.evaluate(&a, &b),
            Some(DuplicatePairDecision::Delete {
                delete_a: true,
                delete_b: false,
            })
        );
        assert_eq!(
            rule.evaluate(&b, &a),
            Some(DuplicatePairDecision::Delete {
                delete_a: false,
                delete_b: true,
            })
        );
    }

    #[test]
    fn resolution_rule_compares_pixel_count() {
        let rule = DuplicatesAutoResolutionRule::new(
            "res",
            DuplicateComparator::HigherResolution,
            DuplicatePairDecision::Approve,
        );

        assert_eq!(
            rule.evaluate(&media(1, (100, 100)), &media(1, (10, 10))),
            Some(DuplicatePairDecision::Approve)
        );
    }

    #[test]
    fn tag_count_rule_uses_all_services() {
        let mut a = media(1, (10, 10));
        a.tags_manager
            .tags
            .add_tags(b"local tags", ["a", "b"], TagStatus::Current);
        let b = media(1, (10, 10));

        let rule = DuplicatesAutoResolutionRule::new(
            "tags",
            DuplicateComparator::MoreTags,
            DuplicatePairDecision::Approve,
        );

        assert_eq!(
            rule.evaluate(&a, &b),
            Some(DuplicatePairDecision::Approve)
        );
        // The decision is orientation-independent: the better file is still
        // identified no matter which side of the pair it sits on.
        assert_eq!(
            rule.evaluate(&b, &a),
            Some(DuplicatePairDecision::Approve)
        );

        // Equal tag counts mean the rule cannot decide.
        assert_eq!(rule.evaluate(&b, &b), None);
    }

    #[test]
    fn manager_uses_first_matching_rule() {
        let mut manager = DuplicatesAutoResolutionManager::new();
        manager.add_rule(DuplicatesAutoResolutionRule::new(
            "always",
            DuplicateComparator::Always,
            DuplicatePairDecision::Skip,
        ));

        assert_eq!(
            manager.evaluate(&media(1, (1, 1)), &media(2, (2, 2))),
            Some(DuplicatePairDecision::Skip)
        );
        assert!(manager.remove_rule("always"));
        assert!(manager.is_empty());
    }
}
