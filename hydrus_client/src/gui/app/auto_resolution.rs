//! User-configurable duplicate auto-resolution rules.
//!
//! A rule narrows the duplicate queue with an optional search, then tests each
//! pair against a selector. The selector is an AND list of comparators; a
//! comparator can be a one-file condition, a relative A/B comparison, a
//! filetype check or a nested AND/OR group. Pairs are tested both ways around,
//! exactly like `PairSelector.GetMatchingAB` in the Python client.
//!
//! Rules run either fully automatically or collect matches into a review queue
//! (`duplicate auto-resolution pending.json`) for approval or denial. The
//! review file also remembers denied and actioned pairs so a refresh does not
//! keep offering the same work.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use hydrus_core::hydrus_constants::{general_mimetype_for, Mime, TimestampType};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::controller::Controller;
use crate::db::files_timestamps;

use super::query;

pub const MAX_PENDING_LIMIT: i64 = 10_000;
pub const MAX_ACTIONED_HISTORY: usize = 500;
pub const PREVIEW_SAMPLES: usize = 8;

const OPERATION_MODE_SEMI_AUTOMATIC: &str = "semi-automatic";
const OPERATION_MODE_FULLY_AUTOMATIC: &str = "fully automatic";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Comparator {
    /// A condition on one side of the pair (`looking_at`: `a`, `b`, `either`).
    OneFile {
        #[serde(default = "default_looking_at")]
        looking_at: String,
        property: String,
        #[serde(default = "default_operator")]
        operator: String,
        #[serde(default)]
        value: f64,
        /// Mime/filetype name for the `mime` property.
        #[serde(default)]
        text: String,
    },
    /// Compares a property of A against the same property of B, optionally
    /// scaled and offset: `A <op> B * multiplier + delta`.
    Relative {
        property: String,
        #[serde(default = "default_operator")]
        operator: String,
        #[serde(default = "default_multiplier")]
        multiplier: f64,
        #[serde(default)]
        delta: f64,
    },
    /// A and B share (or do not share) a filetype.
    Filetype {
        #[serde(default = "default_true")]
        same: bool,
    },
    /// Nested AND/OR group of comparators.
    Group {
        #[serde(default = "default_all")]
        mode: String,
        #[serde(default)]
        comparators: Vec<Comparator>,
    },
}

fn default_looking_at() -> String {
    "a".to_string()
}

fn default_operator() -> String {
    ">".to_string()
}

fn default_multiplier() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_all() -> String {
    "all".to_string()
}

impl Comparator {
    fn matches(&self, controller: &Controller, a: &[u8], b: &[u8]) -> bool {
        match self {
            Comparator::OneFile {
                looking_at,
                property,
                operator,
                value,
                text,
            } => {
                let check = |hash: &[u8]| {
                    one_file_matches(controller, hash, property, operator, *value, text)
                };

                match looking_at.as_str() {
                    "b" => check(b),
                    "either" => check(a) || check(b),
                    _ => check(a),
                }
            }
            Comparator::Relative {
                property,
                operator,
                multiplier,
                delta,
            } => {
                let (Some(left), Some(right)) = (
                    property_value(controller, a, property),
                    property_value(controller, b, property),
                ) else {
                    return false;
                };

                compare_numbers(left, operator, right * multiplier + delta)
            }
            Comparator::Filetype { same } => {
                let (Some(left), Some(right)) = (
                    query::file_info_for(controller, a).map(|info| info.mime),
                    query::file_info_for(controller, b).map(|info| info.mime),
                ) else {
                    return false;
                };

                if *same {
                    left == right
                } else {
                    left != right
                }
            }
            Comparator::Group { mode, comparators } => {
                if mode == "any" {
                    comparators
                        .iter()
                        .any(|comparator| comparator.matches(controller, a, b))
                } else {
                    !comparators.is_empty()
                        && comparators
                            .iter()
                            .all(|comparator| comparator.matches(controller, a, b))
                }
            }
        }
    }

    fn summary(&self) -> String {
        match self {
            Comparator::OneFile {
                looking_at,
                property,
                operator,
                value,
                text,
            } => {
                let target = match looking_at.as_str() {
                    "b" => "B",
                    "either" => "either file",
                    _ => "A",
                };
                let operator = if property == "mime" {
                    match operator.as_str() {
                        "!=" => "is not",
                        _ => "is",
                    }
                    .to_string()
                } else {
                    operator.clone()
                };
                let value = if property == "mime" {
                    text.clone()
                } else {
                    pretty_number(*value, property)
                };

                format!("{target} has {property} {operator} {value}")
            }
            Comparator::Relative {
                property,
                operator,
                multiplier,
                delta,
            } => {
                let mut right = "B".to_string();

                if (*multiplier - 1.0).abs() > f64::EPSILON {
                    right = format!("{multiplier:.2}x {right}");
                }

                if *delta > 0.0 {
                    right = format!("{right} +{}", pretty_number(*delta, property));
                } else if *delta < 0.0 {
                    right = format!("{right} -{}", pretty_number(-delta, property));
                }

                format!("A has {property} {operator} {right}")
            }
            Comparator::Filetype { same } => {
                if *same {
                    "A and B have the same filetype".to_string()
                } else {
                    "A and B have different filetypes".to_string()
                }
            }
            Comparator::Group { mode, comparators } => {
                if comparators.is_empty() {
                    return format!("empty {mode} group (never matches)");
                }

                let parts: Vec<String> = comparators.iter().map(Comparator::summary).collect();

                if mode == "any" {
                    format!("({})", parts.join(" OR "))
                } else {
                    parts.join(" AND ")
                }
            }
        }
    }
}

fn one_file_matches(
    controller: &Controller,
    hash: &[u8],
    property: &str,
    operator: &str,
    value: f64,
    text: &str,
) -> bool {
    if property == "mime" {
        let Some(info) = query::file_info_for(controller, hash) else {
            return false;
        };
        let matched = mime_matches(info.mime, text);

        return if operator == "!=" { !matched } else { matched };
    }

    let Some(left) = property_value(controller, hash, property) else {
        return false;
    };

    compare_numbers(left, operator, value)
}

fn mime_matches(mime: Mime, wanted: &str) -> bool {
    let wanted = wanted.trim().to_ascii_lowercase();

    if wanted.is_empty() {
        return false;
    }

    if let Some(wanted_mime) = Mime::from_nice_string(&wanted) {
        if mime == wanted_mime || general_mimetype_for(mime) == Some(wanted_mime) {
            return true;
        }

        if wanted_mime == Mime::GeneralFile {
            return true;
        }
    }

    if let Some(lookup) = Mime::from_mimetype(&wanted) {
        if lookup.contains(mime) {
            return true;
        }
    }

    false
}

fn property_value(controller: &Controller, hash: &[u8], property: &str) -> Option<f64> {
    match property {
        "archived_days" => timestamp_age_days(controller, hash, TimestampType::Archived),
        "imported_days" => timestamp_age_days(controller, hash, TimestampType::Imported),
        "last_viewed_days" => timestamp_age_days(controller, hash, TimestampType::LastViewed),
        _ => {
            let info = query::file_info_for(controller, hash)?;

            match property {
                "size" => Some(info.size as f64),
                "width" => info.width.map(f64::from),
                "height" => info.height.map(f64::from),
                "ratio" => {
                    let width = info.width?;
                    let height = info.height?;

                    (height > 0).then(|| f64::from(width) / f64::from(height))
                }
                "num_pixels" => Some(f64::from(info.width?) * f64::from(info.height?)),
                "num_tags" => Some(query::selection_tags(controller, hash).len() as f64),
                "num_urls" => Some(query::urls_for(controller, hash).len() as f64),
                "has_audio" => Some(if info.has_audio { 1.0 } else { 0.0 }),
                "duration" => info.duration_ms.map(|duration| duration as f64 / 1000.0),
                "num_frames" => info.num_frames.map(f64::from),
                "num_words" => info.num_words.map(|value| value as f64),
                _ => None,
            }
        }
    }
}

fn timestamp_age_days(
    controller: &Controller,
    hash: &[u8],
    timestamp_type: TimestampType,
) -> Option<f64> {
    let hash_id = query::hash_id_for(controller, hash)?;
    let timestamp =
        files_timestamps::get_file_timestamp(controller.db().db(), hash_id, timestamp_type)
            .ok()
            .flatten()?;
    let now = hydrus_core::hydrus_time::get_now() as f64;

    Some(((now - timestamp as f64) / 86_400.0).max(0.0))
}

fn compare_numbers(left: f64, operator: &str, right: f64) -> bool {
    match operator {
        ">" => left > right,
        "<" => left < right,
        ">=" => left >= right,
        "<=" => left <= right,
        "!=" => (left - right).abs() > f64::EPSILON,
        "=" | "==" => (left - right).abs() <= f64::EPSILON,
        _ => false,
    }
}

fn pretty_number(value: f64, property: &str) -> String {
    match property {
        "size" => human_bytes(value.max(0.0) as u64),
        "duration" => format!("{value:.2}s"),
        "has_audio" => {
            if value >= 1.0 {
                "yes".to_string()
            } else {
                "no".to_string()
            }
        }
        "ratio" => format!("{value:.3}"),
        _ if (value.fract()).abs() <= f64::EPSILON => format!("{}", value as i64),
        _ => format!("{value:.2}"),
    }
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoResolutionRule {
    pub name: String,
    #[serde(default)]
    pub paused: bool,
    #[serde(default = "default_operation_mode")]
    pub operation_mode: String,
    #[serde(default = "default_max_pending_pairs")]
    pub max_pending_pairs: i64,
    #[serde(default)]
    pub search_query: String,
    #[serde(default = "default_search_mode")]
    pub search_mode: String,
    /// Zero means "use the page/global distance"; a positive value narrows it.
    #[serde(default)]
    pub max_distance: i64,
    #[serde(default)]
    pub pair_selector: Vec<Comparator>,
    #[serde(default = "default_action")]
    pub action: String,
    #[serde(default)]
    pub delete_a: bool,
    #[serde(default)]
    pub delete_b: bool,
    /// Legacy v1 field, only read when loading old rule files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparator: Option<String>,
}

fn default_operation_mode() -> String {
    OPERATION_MODE_SEMI_AUTOMATIC.to_string()
}

fn default_max_pending_pairs() -> i64 {
    500
}

fn default_search_mode() -> String {
    "both".to_string()
}

fn default_action() -> String {
    "better".to_string()
}

impl Default for AutoResolutionRule {
    fn default() -> Self {
        Self {
            name: "new rule".to_string(),
            paused: false,
            operation_mode: default_operation_mode(),
            max_pending_pairs: default_max_pending_pairs(),
            search_query: String::new(),
            search_mode: default_search_mode(),
            max_distance: 0,
            pair_selector: vec![Comparator::Relative {
                property: "num_pixels".to_string(),
                operator: ">".to_string(),
                multiplier: 1.0,
                delta: 0.0,
            }],
            action: default_action(),
            delete_a: false,
            delete_b: false,
            comparator: None,
        }
    }
}

impl AutoResolutionRule {
    pub fn normalise(&mut self) {
        self.name = self.name.trim().to_string();

        if self.name.is_empty() {
            self.name = "new rule".to_string();
        }

        if self.operation_mode != OPERATION_MODE_FULLY_AUTOMATIC {
            self.operation_mode = OPERATION_MODE_SEMI_AUTOMATIC.to_string();
        }

        self.max_pending_pairs = self.max_pending_pairs.clamp(1, MAX_PENDING_LIMIT);

        if self.search_mode != "either" {
            self.search_mode = "both".to_string();
        }

        self.max_distance = self.max_distance.max(0);
        self.action = match self.action.as_str() {
            "worse" => "worse",
            "same" => "same",
            "alternates" => "alternates",
            "not_duplicates" => "not_duplicates",
            "delete_both" => "delete_both",
            _ => "better",
        }
        .to_string();

        if self.pair_selector.is_empty() {
            if let Some(comparator) = self.comparator.take() {
                self.pair_selector = legacy_selector(&comparator);
            }
        }

        let action = match self.action.as_str() {
            "keep_better" => "better",
            other => other,
        };

        if action != self.action {
            self.action = action.to_string();
        }
    }

    pub fn prepare<'a>(&'a self, controller: &Controller) -> PreparedRule<'a> {
        let mut search = None;
        let mut search_error = None;

        if !self.search_query.trim().is_empty() {
            match query::hashes_for(controller, &self.search_query, "my files") {
                Ok(hashes) => {
                    search = Some(hashes.into_iter().collect::<BTreeSet<Vec<u8>>>());
                }
                Err(error) => {
                    search_error = Some(error);
                }
            }
        }

        PreparedRule {
            rule: self,
            search,
            search_error,
        }
    }

    pub fn selector_summary(&self) -> String {
        if self.pair_selector.is_empty() {
            return "matches every pair".to_string();
        }

        self.pair_selector
            .iter()
            .map(Comparator::summary)
            .collect::<Vec<String>>()
            .join(" AND ")
    }

    pub fn action_summary(&self) -> String {
        let mut summary = match self.action.as_str() {
            "worse" => "B is the better duplicate; A is moved to trash",
            "same" => "mark as duplicates and keep both",
            "alternates" => "mark as alternates",
            "not_duplicates" => "mark as not duplicates",
            "delete_both" => "move both files to trash",
            _ => "A is the better duplicate; B is moved to trash",
        }
        .to_string();

        if self.action != "better" && self.action != "worse" {
            if self.delete_a {
                summary.push_str("; delete A");
            }

            if self.delete_b {
                summary.push_str("; delete B");
            }
        }

        summary
    }

    pub fn search_summary(&self) -> String {
        let mut summary = if self.search_query.trim().is_empty() {
            "all potential duplicate pairs".to_string()
        } else if self.search_mode == "either" {
            format!("either file matches '{}'", self.search_query.trim())
        } else {
            format!("both files match '{}'", self.search_query.trim())
        };

        if self.max_distance > 0 {
            summary.push_str(&format!(", distance <= {}", self.max_distance));
        }

        summary
    }

    fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "paused": self.paused,
            "operation_mode": self.operation_mode,
            "max_pending_pairs": self.max_pending_pairs,
            "search_query": self.search_query,
            "search_mode": self.search_mode,
            "max_distance": self.max_distance,
            "pair_selector": self.pair_selector,
            "action": self.action,
            "delete_a": self.delete_a,
            "delete_b": self.delete_b,
            "selector_summary": self.selector_summary(),
            "action_summary": self.action_summary(),
            "search_summary": self.search_summary(),
            "operation_label": if self.operation_mode == OPERATION_MODE_FULLY_AUTOMATIC {
                "fully automatic"
            } else {
                "semi-automatic"
            },
        })
    }
}

fn legacy_selector(comparator: &str) -> Vec<Comparator> {
    let relative = |property: &str, operator: &str| Comparator::Relative {
        property: property.to_string(),
        operator: operator.to_string(),
        multiplier: 1.0,
        delta: 0.0,
    };

    match comparator {
        "higher_resolution" => vec![relative("num_pixels", ">")],
        "larger_file" => vec![relative("size", ">")],
        "more_tags" => vec![relative("num_tags", ">")],
        "fewer_tags" => vec![relative("num_tags", "<")],
        "older" => vec![relative("archived_days", ">")],
        _ => Vec::new(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoDecision {
    KeepFirst,
    KeepSecond,
    Same,
    Alternates,
    NotDuplicates,
    DeleteBoth,
}

impl AutoDecision {
    pub fn label(self) -> &'static str {
        match self {
            AutoDecision::KeepFirst => "keep A, trash B",
            AutoDecision::KeepSecond => "keep B, trash A",
            AutoDecision::Same => "they are the same",
            AutoDecision::Alternates => "they are alternates",
            AutoDecision::NotDuplicates => "they are not duplicates",
            AutoDecision::DeleteBoth => "delete both",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleMatch {
    pub decision: AutoDecision,
    pub delete_a: bool,
    pub delete_b: bool,
}

pub struct PreparedRule<'a> {
    rule: &'a AutoResolutionRule,
    search: Option<BTreeSet<Vec<u8>>>,
    search_error: Option<String>,
}

impl PreparedRule<'_> {
    pub fn rule(&self) -> &AutoResolutionRule {
        self.rule
    }

    pub fn search_error(&self) -> Option<&str> {
        self.search_error.as_deref()
    }

    pub fn matches_search(&self, a: &[u8], b: &[u8]) -> bool {
        let Some(search) = &self.search else {
            return self.search_error.is_none();
        };
        let a_in = search.contains(a);
        let b_in = search.contains(b);

        if self.rule.search_mode == "either" {
            a_in || b_in
        } else {
            a_in && b_in
        }
    }

    pub fn evaluate(
        &self,
        controller: &Controller,
        a: &[u8],
        b: &[u8],
        distance: i64,
    ) -> Option<RuleMatch> {
        if self.rule.paused {
            return None;
        }

        if self.rule.max_distance > 0 && distance > self.rule.max_distance {
            return None;
        }

        if !self.matches_search(a, b) {
            return None;
        }

        let swapped = if self.rule.pair_selector.is_empty() {
            false
        } else if self
            .rule
            .pair_selector
            .iter()
            .all(|comparator| comparator.matches(controller, a, b))
        {
            false
        } else if self
            .rule
            .pair_selector
            .iter()
            .all(|comparator| comparator.matches(controller, b, a))
        {
            true
        } else {
            return None;
        };

        let decision = match self.rule.action.as_str() {
            "worse" => {
                if swapped {
                    AutoDecision::KeepFirst
                } else {
                    AutoDecision::KeepSecond
                }
            }
            "same" => AutoDecision::Same,
            "alternates" => AutoDecision::Alternates,
            "not_duplicates" => AutoDecision::NotDuplicates,
            "delete_both" => AutoDecision::DeleteBoth,
            _ => {
                if swapped {
                    AutoDecision::KeepSecond
                } else {
                    AutoDecision::KeepFirst
                }
            }
        };

        Some(RuleMatch {
            decision,
            delete_a: self.rule.delete_a,
            delete_b: self.rule.delete_b,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingDecision {
    pub rule: String,
    pub a: String,
    pub b: String,
    #[serde(default)]
    pub distance: i64,
    pub decision: AutoDecision,
    #[serde(default)]
    pub delete_a: bool,
    #[serde(default)]
    pub delete_b: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeniedDecision {
    pub rule: String,
    pub a: String,
    pub b: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionedDecision {
    pub rule: String,
    pub a: String,
    pub b: String,
    pub decision: AutoDecision,
    #[serde(default)]
    pub banned: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DuplicateReviewState {
    #[serde(default)]
    pub pending: Vec<PendingDecision>,
    #[serde(default)]
    pub denied: Vec<DeniedDecision>,
    #[serde(default)]
    pub actioned: Vec<ActionedDecision>,
}

impl DuplicateReviewState {
    pub fn is_denied(&self, rule: &str, a: &[u8], b: &[u8]) -> bool {
        let (a, b) = unordered_hex(a, b);

        self.denied
            .iter()
            .any(|entry| entry.rule == rule && entry.a == a && entry.b == b)
    }

    /// Queues a matched pair for review. Returns false when the rule is
    /// already at its cap, the pair was denied already or the pair is pending.
    pub fn queue(
        &mut self,
        rule: &AutoResolutionRule,
        a: &[u8],
        b: &[u8],
        distance: i64,
        matched: RuleMatch,
    ) -> bool {
        if self.is_denied(&rule.name, a, b) {
            return false;
        }

        let (unordered_a, unordered_b) = unordered_hex(a, b);

        if self.pending.iter().any(|entry| {
            let (entry_a, entry_b) = if entry.a <= entry.b {
                (entry.a.as_str(), entry.b.as_str())
            } else {
                (entry.b.as_str(), entry.a.as_str())
            };

            entry.rule == rule.name
                && entry_a == unordered_a.as_str()
                && entry_b == unordered_b.as_str()
        }) {
            return false;
        }

        let count = self
            .pending
            .iter()
            .filter(|entry| entry.rule == rule.name)
            .count() as i64;

        if count >= rule.max_pending_pairs {
            return false;
        }

        // Keep the matched A/B order: keep_first and keep_second depend on it.
        self.pending.push(PendingDecision {
            rule: rule.name.clone(),
            a: query::hex(a),
            b: query::hex(b),
            distance,
            decision: matched.decision,
            delete_a: matched.delete_a,
            delete_b: matched.delete_b,
        });

        true
    }

    pub fn deny(&mut self, index: usize) {
        if index >= self.pending.len() {
            return;
        }

        let entry = self.pending.remove(index);

        let (a, b) = if entry.a <= entry.b {
            (entry.a, entry.b)
        } else {
            (entry.b, entry.a)
        };

        if !self
            .denied
            .iter()
            .any(|denied| denied.rule == entry.rule && denied.a == a && denied.b == b)
        {
            self.denied.push(DeniedDecision {
                rule: entry.rule,
                a,
                b,
            });
        }
    }

    pub fn record_action(&mut self, index: usize, banned: bool) {
        if index >= self.pending.len() {
            return;
        }

        let entry = self.pending.remove(index);

        self.actioned.push(ActionedDecision {
            rule: entry.rule,
            a: entry.a,
            b: entry.b,
            decision: entry.decision,
            banned,
        });

        if self.actioned.len() > MAX_ACTIONED_HISTORY {
            let overflow = self.actioned.len() - MAX_ACTIONED_HISTORY;
            self.actioned.drain(0..overflow);
        }
    }
}

fn unordered_hex(a: &[u8], b: &[u8]) -> (String, String) {
    let (left, right) = if a <= b { (a, b) } else { (b, a) };

    (query::hex(left), query::hex(right))
}

pub fn review_path(controller: &Controller) -> PathBuf {
    controller
        .db_dir()
        .join("duplicate auto-resolution pending.json")
}

pub fn load_review(path: &Path) -> DuplicateReviewState {
    let Ok(text) = std::fs::read_to_string(path) else {
        return DuplicateReviewState::default();
    };

    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_review(path: &Path, review: &DuplicateReviewState) -> std::io::Result<()> {
    let dump = serde_json::to_string_pretty(review).map_err(std::io::Error::other)?;
    std::fs::write(path, dump)
}

pub fn save_state_review(state: &AppState) -> std::io::Result<()> {
    save_review(&review_path(&state.controller), &state.duplicate_review)
}

pub fn load_rules(path: &Path) -> Vec<AutoResolutionRule> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(values) = serde_json::from_str::<Vec<Value>>(&text) else {
        return Vec::new();
    };

    let mut rules: Vec<AutoResolutionRule> = values
        .into_iter()
        .filter_map(|value| serde_json::from_value::<AutoResolutionRule>(value).ok())
        .map(|mut rule| {
            rule.normalise();
            rule
        })
        .collect();

    sort_rules(&mut rules);

    rules
}

pub fn save_rules(path: &Path, rules: &[AutoResolutionRule]) -> std::io::Result<()> {
    let dump = serde_json::to_string_pretty(rules).map_err(std::io::Error::other)?;
    std::fs::write(path, dump)
}

fn sort_rules(rules: &mut [AutoResolutionRule]) {
    rules.sort_by_key(|rule| rule.name.to_lowercase());
}

pub fn catalog_json() -> Value {
    json!({
        "properties": [
            {"value":"mime","label":"mime / filetype","text":true},
            {"value":"size","label":"size"},
            {"value":"width","label":"width"},
            {"value":"height","label":"height"},
            {"value":"ratio","label":"width / height ratio"},
            {"value":"num_pixels","label":"number of pixels"},
            {"value":"num_tags","label":"number of tags"},
            {"value":"num_urls","label":"number of URLs"},
            {"value":"has_audio","label":"has audio (1/0)"},
            {"value":"duration","label":"duration (seconds)"},
            {"value":"num_frames","label":"number of frames"},
            {"value":"num_words","label":"number of words"},
            {"value":"archived_days","label":"days since archived"},
            {"value":"imported_days","label":"days since import"},
            {"value":"last_viewed_days","label":"days since last viewed"}
        ],
        "operators": [
            {"value":">","label":">"},
            {"value":"<","label":"<"},
            {"value":">=","label":">="},
            {"value":"<=","label":"<="},
            {"value":"=","label":"="},
            {"value":"!=","label":"!="}
        ],
        "looking_at": [
            {"value":"a","label":"A"},
            {"value":"b","label":"B"},
            {"value":"either","label":"either file"}
        ],
        "search_modes": [
            {"value":"both","label":"both files match"},
            {"value":"either","label":"either file matches"}
        ],
        "group_modes": [
            {"value":"all","label":"all (AND)"},
            {"value":"any","label":"any (OR)"}
        ],
        "actions": [
            {"value":"better","label":"A is the better duplicate (trash B)"},
            {"value":"worse","label":"B is the better duplicate (trash A)"},
            {"value":"same","label":"they are the same (keep both)"},
            {"value":"alternates","label":"they are alternates"},
            {"value":"not_duplicates","label":"they are not duplicates"},
            {"value":"delete_both","label":"delete both"}
        ],
        "operation_modes": [
            {"value":"semi-automatic","label":"semi-automatic: collect matches for review"},
            {"value":"fully automatic","label":"fully automatic: apply matches immediately"}
        ]
    })
}

fn rule_json(rule: &AutoResolutionRule) -> Value {
    let mut value = rule.to_json();
    value["comparator"] = json!(rule.comparator);
    value
}

pub fn rules_json(rules: &[AutoResolutionRule], pending: usize) -> String {
    let mut catalog = catalog_json();
    catalog["rules"] = json!(rules.iter().map(rule_json).collect::<Vec<Value>>());
    catalog["pending"] = json!(pending);

    catalog.to_string()
}

pub fn dialog_json(state: &AppState) -> String {
    let mut value: Value = serde_json::from_str(&rules_json(
        &state.auto_rules,
        state.duplicate_review.pending.len(),
    ))
    .unwrap_or_default();
    value["enabled"] = json!(state
        .controller
        .options()
        .get_boolean("duplicates_auto_resolution_enabled"));
    value["denied"] = json!(state.duplicate_review.denied.len());
    value["actioned"] = json!(state.duplicate_review.actioned.len());

    value.to_string()
}

pub fn save_rule(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(mut value) = serde_json::from_str::<Value>(payload) else {
        return notify(state, "invalid auto-resolution rule");
    };

    let original_name = value
        .get("original_name")
        .and_then(Value::as_str)
        .map(str::to_string);
    let Some(object) = value.as_object_mut() else {
        return notify(state, "invalid auto-resolution rule");
    };
    object.remove("original_name");

    let Ok(mut rule) = serde_json::from_value::<AutoResolutionRule>(value) else {
        return notify(state, "the auto-resolution rule could not be read");
    };

    rule.normalise();
    rule.name = unique_name(&state.auto_rules, &rule.name, original_name.as_deref());

    if let Some(original) = &original_name {
        state
            .auto_rules
            .retain(|existing| &existing.name != original);

        if original != &rule.name {
            for entry in &mut state.duplicate_review.pending {
                if &entry.rule == original {
                    entry.rule = rule.name.clone();
                }
            }

            for entry in &mut state.duplicate_review.denied {
                if &entry.rule == original {
                    entry.rule = rule.name.clone();
                }
            }

            for entry in &mut state.duplicate_review.actioned {
                if &entry.rule == original {
                    entry.rule = rule.name.clone();
                }
            }
        }
    }

    state
        .auto_rules
        .retain(|existing| existing.name != rule.name);
    state.auto_rules.push(rule.clone());
    sort_rules(&mut state.auto_rules);
    let _ = persist(state);
    state.message = format!("auto-resolution rule '{}' saved", rule.name);

    refresh(state)
}

pub fn remove_rule(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let name = payload.trim();

    state.auto_rules.retain(|rule| rule.name != name);
    state
        .duplicate_review
        .pending
        .retain(|entry| entry.rule != name);
    state
        .duplicate_review
        .denied
        .retain(|entry| entry.rule != name);
    let _ = persist(state);
    let _ = save_state_review(state);
    state.message = format!("auto-resolution rule '{name}' removed");

    refresh(state)
}

pub fn persist(state: &AppState) -> std::io::Result<()> {
    save_rules(
        &state
            .controller
            .db_dir()
            .join("duplicate auto-resolution rules.json"),
        &state.auto_rules,
    )
}

fn unique_name(rules: &[AutoResolutionRule], wanted: &str, original_name: Option<&str>) -> String {
    let wanted = wanted.trim();
    let wanted = if wanted.is_empty() {
        "new rule"
    } else {
        wanted
    };
    let mut candidate = wanted.to_string();
    let mut index = 2;

    loop {
        let clash = rules.iter().any(|rule| {
            rule.name.eq_ignore_ascii_case(&candidate)
                && original_name.map_or(true, |original| rule.name != original)
        });

        if !clash {
            return candidate;
        }

        candidate = format!("{wanted} ({index})");
        index += 1;
    }
}

pub fn preview_rule(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(mut value) = serde_json::from_str::<Value>(payload) else {
        return notify(state, "invalid auto-resolution rule");
    };

    if let Some(object) = value.as_object_mut() {
        object.remove("original_name");
    }

    let Ok(mut rule) = serde_json::from_value::<AutoResolutionRule>(value) else {
        return notify(state, "the auto-resolution rule could not be read");
    };

    rule.normalise();

    let max_distance = if rule.max_distance > 0 {
        Some(rule.max_distance)
    } else {
        let setting = state
            .controller
            .options()
            .get_integer("similar_files_duplicate_pairs_search_distance");
        (setting > 0).then_some(setting)
    };

    let prepared = rule.prepare(&state.controller);
    let error = prepared.search_error().map(str::to_string);
    let mut total = 0usize;
    let mut matched = 0usize;
    let mut failed = 0usize;
    let mut samples = Vec::new();

    match crate::db::duplicates::get_candidate_pairs(state.controller.db().db(), max_distance) {
        Ok(pairs) => {
            for (smaller, larger, distance) in pairs {
                let Some(a) = crate::db::master::get_hash(state.controller.db().db(), smaller)
                    .ok()
                    .flatten()
                else {
                    continue;
                };
                let Some(b) = crate::db::master::get_hash(state.controller.db().db(), larger)
                    .ok()
                    .flatten()
                else {
                    continue;
                };

                total += 1;

                if let Some(result) = prepared.evaluate(&state.controller, &a, &b, distance) {
                    matched += 1;

                    if samples.len() < PREVIEW_SAMPLES {
                        let discarded = match result.decision {
                            AutoDecision::KeepFirst => &b,
                            AutoDecision::KeepSecond => &a,
                            _ => &b,
                        };
                        let kept = match result.decision {
                            AutoDecision::KeepFirst => &a,
                            AutoDecision::KeepSecond => &b,
                            _ => &a,
                        };

                        samples.push(json!({
                            "a": query::hex(&a),
                            "b": query::hex(&b),
                            "distance": distance,
                            "decision": result.decision.label(),
                            "kept": query::hex(kept),
                            "discarded": query::hex(discarded),
                        }));
                    }
                } else {
                    failed += 1;
                }
            }
        }
        Err(error) => {
            return notify(
                state,
                format!("could not read the duplicate queue: {error}"),
            );
        }
    }

    let projected = if rule.operation_mode == OPERATION_MODE_FULLY_AUTOMATIC {
        "would be applied immediately"
    } else {
        "would be queued for review"
    };
    let status = if let Some(error) = &error {
        format!("search failed: {error}")
    } else {
        format!("{matched} of {total} pairs match; {failed} do not; matches {projected}")
    };

    vec![UiCommand::DuplicatesRulePreview(
        json!({
            "rule": rule.name,
            "total": total,
            "matched": matched,
            "failed": failed,
            "samples": samples,
            "status": status,
            "error": error,
            "selector": rule.selector_summary(),
            "search": rule.search_summary(),
        })
        .to_string(),
    )]
}

pub fn review_json(state: &AppState) -> String {
    let review = &state.duplicate_review;
    let pending: Vec<Value> = review
        .pending
        .iter()
        .map(|entry| {
            json!({
                "rule": entry.rule,
                "a": entry.a,
                "b": entry.b,
                "distance": entry.distance,
                "decision": entry.decision,
                "decision_label": entry.decision.label(),
                "delete_a": entry.delete_a,
                "delete_b": entry.delete_b,
            })
        })
        .collect();
    let denied: Vec<Value> = review
        .denied
        .iter()
        .map(|entry| json!({"rule": entry.rule, "a": entry.a, "b": entry.b}))
        .collect();
    let actioned: Vec<Value> = review
        .actioned
        .iter()
        .map(|entry| {
            json!({
                "rule": entry.rule,
                "a": entry.a,
                "b": entry.b,
                "decision": entry.decision,
                "decision_label": entry.decision.label(),
                "banned": entry.banned,
            })
        })
        .collect();

    json!({
        "pending": pending,
        "denied": denied,
        "actioned": actioned,
        "status": match review.pending.len() {
            0 => "no pending auto-resolution pairs".to_string(),
            1 => "1 pending auto-resolution pair".to_string(),
            count => format!("{count} pending auto-resolution pairs"),
        },
    })
    .to_string()
}

pub fn review_open(state: &AppState) -> Vec<UiCommand> {
    vec![UiCommand::DuplicateReviewDialog(review_json(state))]
}

pub fn review_apply(state: &mut AppState, payload: &str) -> Vec<UiCommand> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return notify(state, "invalid review selection");
    };
    let approve = value
        .get("approve")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let all = value.get("all").and_then(Value::as_bool).unwrap_or(false);
    let indices: Vec<usize> = if all {
        (0..state.duplicate_review.pending.len()).collect()
    } else {
        value
            .get("indices")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_u64)
                    .map(|index| index as usize)
                    .collect()
            })
            .unwrap_or_default()
    };

    if indices.is_empty() {
        return vec![UiCommand::NotifyMessage(
            "choose at least one pending pair".to_string(),
        )];
    }

    // Remove in reverse so the stored indices stay valid.
    let mut indices = indices;
    indices.sort_unstable();
    indices.dedup();

    let mut approved = 0usize;
    let mut denied = 0usize;
    let mut failures = 0usize;

    for index in indices.into_iter().rev() {
        if index >= state.duplicate_review.pending.len() {
            continue;
        }

        if approve {
            let entry = state.duplicate_review.pending[index].clone();
            let (Some(a), Some(b)) = (query::unhex(&entry.a), query::unhex(&entry.b)) else {
                failures += 1;
                continue;
            };

            match super::duplicates::apply_rule_decision(
                &state.controller,
                entry.decision,
                entry.delete_a,
                entry.delete_b,
                &a,
                &b,
            ) {
                Ok(()) => {
                    state.duplicate_review.record_action(index, false);
                    approved += 1;
                }
                Err(_) => {
                    failures += 1;
                }
            }
        } else {
            state.duplicate_review.deny(index);
            denied += 1;
        }
    }

    let _ = save_state_review(state);

    state.message = if approve {
        format!("approved {approved} pair(s); {failures} failed")
    } else {
        format!("denied {denied} pair(s)")
    };

    state.refresh_all();

    let mut commands = state.refresh_commands();
    commands.push(UiCommand::UpdateDuplicateReview(review_json(state)));
    commands.push(UiCommand::NotifyMessage(state.message.clone()));

    commands
}

fn refresh(state: &mut AppState) -> Vec<UiCommand> {
    state.refresh_all();

    let mut commands = state.refresh_commands();
    commands.push(UiCommand::DuplicatesRulesDialog(dialog_json(state)));
    commands.push(UiCommand::BuildPages(render::pages_json(state)));
    commands.push(UiCommand::Status(render::status_json(state)));

    commands
}

fn notify(state: &AppState, message: impl Into<String>) -> Vec<UiCommand> {
    vec![
        UiCommand::NotifyMessage(message.into()),
        UiCommand::DuplicatesRulesDialog(dialog_json(state)),
    ]
}

use super::render;
use super::{AppState, UiCommand};

#[cfg(test)]
mod tests;
