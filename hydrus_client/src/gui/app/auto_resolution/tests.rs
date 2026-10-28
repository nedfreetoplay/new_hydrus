use std::sync::Arc;

use hydrus_core::files::hydrus_file_handling::FileInfo;
use hydrus_core::hydrus_constants::{ContentStatus, Mime};

use super::*;
use crate::db::{files_metadata, files_storage, mappings, master, master_tags};
use crate::gui::app::{AppState, UiCommand};

fn temp_directory(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "hydrus_auto_resolution_{label}_{}_{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    std::fs::create_dir_all(&directory).unwrap();

    directory
}

fn test_state(directory: &Path) -> AppState {
    let _ = crate::client_options::register_serialisable_type();
    let controller = Controller::new(directory).unwrap();
    controller.init_model().unwrap();

    AppState::new(Arc::new(controller))
}

struct TestFile {
    hash: Vec<u8>,
    id: i64,
}

fn add_file(
    state: &AppState,
    seed: u8,
    size: u64,
    width: u32,
    height: u32,
    tags: &[&str],
) -> TestFile {
    let db = state.controller.db().db();
    let hash = [seed; 32];
    let id = master::get_hash_id(db, &hash).unwrap();

    files_metadata::set_file_info(
        db,
        id,
        &FileInfo {
            size,
            mime: Mime::Png,
            width: Some(width),
            height: Some(height),
            duration_ms: None,
            num_frames: None,
            has_audio: false,
            num_words: None,
        },
    )
    .unwrap();

    let service = state
        .controller
        .services()
        .get_service(b"local tags")
        .unwrap()
        .service_id;

    for tag in tags {
        let tag_id = master_tags::get_tag_id(db, tag).unwrap();
        mappings::add_mappings(db, service, ContentStatus::Current, id, &[tag_id]).unwrap();
    }

    if let Some(file_service) =
        crate::gui::app::query::local_file_service_id(&state.controller, "my files")
    {
        files_storage::add_files(
            db,
            file_service,
            ContentStatus::Current,
            &[id],
            Some(hydrus_core::hydrus_time::get_now()),
        )
        .unwrap();
    }

    TestFile {
        hash: hash.to_vec(),
        id,
    }
}

fn relative(property: &str, operator: &str) -> Comparator {
    Comparator::Relative {
        property: property.to_string(),
        operator: operator.to_string(),
        multiplier: 1.0,
        delta: 0.0,
    }
}

fn rule_with(selector: Vec<Comparator>, action: &str) -> AutoResolutionRule {
    AutoResolutionRule {
        name: "test rule".to_string(),
        pair_selector: selector,
        action: action.to_string(),
        ..AutoResolutionRule::default()
    }
}

#[test]
fn rules_round_trip_through_json() {
    let rules = vec![
        rule_with(
            vec![
                Comparator::OneFile {
                    looking_at: "a".to_string(),
                    property: "mime".to_string(),
                    operator: "=".to_string(),
                    value: 0.0,
                    text: "image".to_string(),
                },
                Comparator::Group {
                    mode: "any".to_string(),
                    comparators: vec![relative("size", ">"), relative("num_pixels", ">")],
                },
            ],
            "better",
        ),
        AutoResolutionRule {
            name: "paused big".to_string(),
            paused: true,
            search_query: "blue eyes".to_string(),
            ..AutoResolutionRule::default()
        },
    ];

    let text = serde_json::to_string(&rules).unwrap();
    let parsed: Vec<AutoResolutionRule> = serde_json::from_str(&text).unwrap();

    assert_eq!(parsed, rules);
}

#[test]
fn editor_payload_round_trips_and_matches() {
    let payload = r#"{
        "action": "better",
        "delete_a": false,
        "delete_b": false,
        "max_distance": 0,
        "max_pending_pairs": 500,
        "name": "1 - bigger file",
        "operation_mode": "semi-automatic",
        "pair_selector": [
            {
                "comparators": [
                    {"delta": 0, "kind": "relative", "multiplier": 1.1, "operator": ">", "property": "size"},
                    {"kind": "one_file", "looking_at": "a", "operator": "=", "property": "mime", "text": "image", "value": 0}
                ],
                "kind": "group",
                "mode": "any"
            }
        ],
        "paused": false,
        "search_mode": "both",
        "search_query": ""
    }"#;
    let mut rule: AutoResolutionRule = serde_json::from_str(payload).unwrap();
    rule.normalise();

    assert_eq!(
        rule.selector_summary(),
        "(A has size > 1.10x B OR A has mime is image)"
    );

    let directory = temp_directory("editor_payload");
    let state = test_state(&directory);
    let first = add_file(&state, 41, 100_000, 640, 480, &[]);
    let second = add_file(&state, 42, 105_000, 650, 480, &[]);
    let prepared = rule.prepare(&state.controller);

    assert!(prepared
        .evaluate(&state.controller, &first.hash, &second.hash, 1)
        .is_some());

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn mime_groups_are_recognised() {
    assert!(mime_matches(Mime::Png, "image"));
    assert!(mime_matches(Mime::Jpeg, "image"));
    assert!(mime_matches(Mime::Jpeg, "image/jpeg"));
    assert!(mime_matches(Mime::Png, "png"));
    assert!(!mime_matches(Mime::Png, "video"));
}

#[test]
fn legacy_rules_migrate_to_a_selector() {
    let directory = temp_directory("legacy");
    let path = directory.join("duplicate auto-resolution rules.json");
    std::fs::write(
        &path,
        r#"[
            {"name":"larger wins","comparator":"larger_file","action":"keep_better"},
            {"name":"no dupes","comparator":"always","action":"not_duplicates"}
        ]"#,
    )
    .unwrap();

    let rules = load_rules(&path);

    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].action, "better");
    assert_eq!(rules[0].pair_selector, vec![relative("size", ">")]);
    assert!(rules[1].pair_selector.is_empty());
    assert_eq!(rules[1].action, "not_duplicates");

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn relative_selectors_test_both_ways_around() {
    let directory = temp_directory("relative");
    let state = test_state(&directory);
    let small = add_file(&state, 1, 1_000, 640, 480, &[]);
    let large = add_file(&state, 2, 4_000, 1280, 960, &[]);
    let rule = rule_with(vec![relative("size", ">")], "better");
    let prepared = rule.prepare(&state.controller);

    assert_eq!(
        prepared
            .evaluate(&state.controller, &large.hash, &small.hash, 2)
            .map(|result| result.decision),
        Some(AutoDecision::KeepFirst)
    );
    assert_eq!(
        prepared
            .evaluate(&state.controller, &small.hash, &large.hash, 2)
            .map(|result| result.decision),
        Some(AutoDecision::KeepSecond)
    );
    assert!(prepared
        .evaluate(&state.controller, &small.hash, &small.hash, 2)
        .is_none());

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn one_file_and_or_groups_match_metadata() {
    let directory = temp_directory("groups");
    let state = test_state(&directory);
    let wide = add_file(&state, 3, 2_000, 1600, 900, &["blue eyes"]);
    let tall = add_file(&state, 4, 2_000, 600, 1200, &[]);

    let one_file = Comparator::OneFile {
        looking_at: "a".to_string(),
        property: "mime".to_string(),
        operator: "=".to_string(),
        value: 0.0,
        text: "image".to_string(),
    };
    let group = Comparator::Group {
        mode: "any".to_string(),
        comparators: vec![relative("width", ">"), relative("num_tags", ">")],
    };
    let rule = rule_with(vec![one_file, group], "better");
    let prepared = rule.prepare(&state.controller);

    assert!(prepared
        .evaluate(&state.controller, &wide.hash, &tall.hash, 1)
        .is_some());
    assert!(prepared
        .evaluate(&state.controller, &tall.hash, &wide.hash, 1)
        .is_some());

    let impossible = rule_with(
        vec![Comparator::Group {
            mode: "all".to_string(),
            comparators: vec![
                relative("width", ">"),
                Comparator::OneFile {
                    looking_at: "a".to_string(),
                    property: "size".to_string(),
                    operator: "<".to_string(),
                    value: 1_000.0,
                    text: String::new(),
                },
            ],
        }],
        "better",
    );
    let prepared = impossible.prepare(&state.controller);

    assert!(prepared
        .evaluate(&state.controller, &wide.hash, &tall.hash, 1)
        .is_none());

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn search_context_can_require_both_or_either_file() {
    let directory = temp_directory("search");
    let state = test_state(&directory);
    let tagged = add_file(&state, 5, 1_000, 640, 480, &["blue eyes"]);
    let plain = add_file(&state, 6, 2_000, 640, 480, &[]);
    let mut rule = rule_with(vec![relative("size", ">")], "better");
    rule.search_query = "blue eyes".to_string();
    rule.search_mode = "both".to_string();

    let prepared = rule.prepare(&state.controller);
    assert!(prepared
        .evaluate(&state.controller, &plain.hash, &tagged.hash, 1)
        .is_none());

    rule.search_mode = "either".to_string();
    let prepared = rule.prepare(&state.controller);
    assert!(prepared
        .evaluate(&state.controller, &plain.hash, &tagged.hash, 1)
        .is_some());

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn review_queue_respects_caps_and_remembers_denials() {
    let directory = temp_directory("queue");
    let state = test_state(&directory);
    let a = add_file(&state, 7, 1_000, 640, 480, &[]);
    let b = add_file(&state, 8, 2_000, 640, 480, &[]);
    let mut rule = rule_with(vec![relative("size", ">")], "same");
    rule.name = "semi".to_string();
    rule.max_pending_pairs = 1;
    let matched = RuleMatch {
        decision: AutoDecision::Same,
        delete_a: false,
        delete_b: false,
    };
    let mut review = DuplicateReviewState::default();

    assert!(review.queue(&rule, &b.hash, &a.hash, 3, matched));
    assert!(!review.queue(&rule, &b.hash, &a.hash, 3, matched));
    assert!(!review.queue(&rule, &a.hash, &b.hash, 3, matched));
    assert_eq!(review.pending.len(), 1);
    assert_eq!(review.pending[0].a, query::hex(&b.hash));
    assert_eq!(review.pending[0].b, query::hex(&a.hash));

    review.deny(0);
    assert!(review.pending.is_empty());
    assert!(review.is_denied(&rule.name, &b.hash, &a.hash));
    assert!(!review.queue(&rule, &b.hash, &a.hash, 3, matched));

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn preview_reports_matches_without_touching_the_database() {
    let directory = temp_directory("preview");
    let mut state = test_state(&directory);
    let a = add_file(&state, 9, 1_000, 640, 480, &[]);
    let b = add_file(&state, 10, 3_000, 640, 480, &[]);
    crate::db::duplicates::add_potential_pair(
        state.controller.db().db(),
        a.id.min(b.id),
        a.id.max(b.id),
        2,
    )
    .unwrap();

    let rule = rule_with(vec![relative("size", ">")], "better");
    let payload = serde_json::to_string(&rule).unwrap();
    let commands = preview_rule(&mut state, &payload);
    let json = commands
        .iter()
        .find_map(|command| match command {
            UiCommand::DuplicatesRulePreview(json) => Some(json.clone()),
            _ => None,
        })
        .expect("preview");
    let value: Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["total"], 1);
    assert_eq!(value["matched"], 1);
    assert_eq!(value["failed"], 0);
    assert_eq!(value["samples"].as_array().unwrap().len(), 1);
    assert!(state.duplicate_review.pending.is_empty());

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn approving_and_denying_pending_pairs_updates_the_queue() {
    let directory = temp_directory("review_apply");
    let mut state = test_state(&directory);
    let small = add_file(&state, 51, 1_000, 640, 480, &[]);
    let large = add_file(&state, 52, 5_000, 640, 480, &[]);
    crate::db::duplicates::add_potential_pair(
        state.controller.db().db(),
        small.id.min(large.id),
        small.id.max(large.id),
        1,
    )
    .unwrap();

    let mut rule = rule_with(vec![relative("size", ">")], "better");
    rule.name = "semi review".to_string();
    let matched = RuleMatch {
        decision: AutoDecision::KeepFirst,
        delete_a: false,
        delete_b: false,
    };

    assert!(state
        .duplicate_review
        .queue(&rule, &large.hash, &small.hash, 1, matched));

    // Denying keeps the pair for manual review and remembers the decision.
    let commands = review_apply(
        &mut state,
        &serde_json::json!({"all": true, "approve": false}).to_string(),
    );
    assert!(state.duplicate_review.pending.is_empty());
    assert_eq!(state.duplicate_review.denied.len(), 1);
    assert_eq!(
        crate::db::duplicates::get_candidate_pairs(state.controller.db().db(), None)
            .unwrap()
            .len(),
        1
    );
    assert!(commands
        .iter()
        .any(|command| matches!(command, UiCommand::NotifyMessage(message) if message.contains("denied 1"))));

    // Re-queuing the denied pair is refused, but a fresh rule can still take it.
    rule.name = "another rule".to_string();
    assert!(state
        .duplicate_review
        .queue(&rule, &large.hash, &small.hash, 1, matched));

    review_apply(
        &mut state,
        &serde_json::json!({"all": true, "approve": true}).to_string(),
    );

    assert!(state.duplicate_review.pending.is_empty());
    assert_eq!(state.duplicate_review.actioned.len(), 1);
    assert!(
        crate::db::duplicates::get_candidate_pairs(state.controller.db().db(), None)
            .unwrap()
            .is_empty()
    );

    let db = state.controller.db().db();
    let my_files = query::local_file_service_id(&state.controller, "my files").unwrap();
    let trash = query::trash_service_id(&state.controller).unwrap();
    assert!(
        files_storage::get_hash_ids(db, my_files, ContentStatus::Current)
            .unwrap()
            .contains(&large.id)
    );
    assert!(
        files_storage::get_hash_ids(db, trash, ContentStatus::Current)
            .unwrap()
            .contains(&small.id)
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn saved_rules_get_unique_names_and_survive_reload() {
    let directory = temp_directory("save");
    let mut state = test_state(&directory);
    let payload = serde_json::to_string(&rule_with(vec![relative("size", ">")], "better")).unwrap();

    save_rule(&mut state, &payload);
    save_rule(&mut state, &payload);

    assert_eq!(state.auto_rules.len(), 2);
    assert!(state.auto_rules.iter().any(|rule| rule.name == "test rule"));
    assert!(state
        .auto_rules
        .iter()
        .any(|rule| rule.name == "test rule (2)"));

    let path = directory.join("duplicate auto-resolution rules.json");
    let reloaded = load_rules(&path);
    assert_eq!(reloaded, state.auto_rules);

    // Renaming the first rule keeps the second one and does not create a copy.
    state.duplicate_review.pending.push(PendingDecision {
        rule: "test rule".to_string(),
        a: "aa".to_string(),
        b: "bb".to_string(),
        distance: 1,
        decision: AutoDecision::Same,
        delete_a: false,
        delete_b: false,
    });
    let mut renamed = rule_with(vec![relative("size", ">")], "better");
    renamed.name = "renamed".to_string();
    let mut value = serde_json::to_value(&renamed).unwrap();
    value["original_name"] = serde_json::json!("test rule");
    save_rule(&mut state, &value.to_string());

    assert_eq!(state.auto_rules.len(), 2);
    assert!(state.auto_rules.iter().any(|rule| rule.name == "renamed"));
    assert!(!state.auto_rules.iter().any(|rule| rule.name == "test rule"));
    assert_eq!(state.duplicate_review.pending[0].rule, "renamed");

    let _ = std::fs::remove_dir_all(directory);
}
