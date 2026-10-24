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

    assert_eq!(rule.evaluate(&a, &b), Some(DuplicatePairDecision::Approve));
    // The decision is orientation-independent: the better file is still
    // identified no matter which side of the pair it sits on.
    assert_eq!(rule.evaluate(&b, &a), Some(DuplicatePairDecision::Approve));

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
