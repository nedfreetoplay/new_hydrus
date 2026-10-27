//! User-configurable duplicate auto-resolution rules.
//!
//! The comparator arithmetic mirrors `duplicates/auto_resolution.rs`, but works
//! directly on the client database instead of materialised `MediaResult`
//! objects. Rules are evaluated in order; the first matching rule decides the
//! pair. Rules only run when the `duplicates_auto_resolution_enabled` option is
//! on.

use std::path::Path;

use hydrus_core::hydrus_constants::TimestampType;
use serde::{Deserialize, Serialize};

use crate::controller::Controller;
use crate::db::files_timestamps;

use super::query;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoResolutionRule {
    pub name: String,
    /// `always`, `higher_resolution`, `larger_file`, `more_tags`, `fewer_tags`,
    /// `older`.
    pub comparator: String,
    /// `keep_better`, `same`, `not_duplicates`, `delete_both`.
    pub action: String,
}

impl Default for AutoResolutionRule {
    fn default() -> Self {
        Self {
            name: "higher resolution wins".to_string(),
            comparator: "higher_resolution".to_string(),
            action: "keep_better".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoDecision {
    KeepFirst,
    KeepSecond,
    Same,
    NotDuplicates,
    DeleteBoth,
}

impl AutoResolutionRule {
    pub fn evaluate(&self, controller: &Controller, a: &[u8], b: &[u8]) -> Option<AutoDecision> {
        let keep_better = self.action == "keep_better";

        let ordering = compare(controller, &self.comparator, a, b);

        if keep_better {
            return match ordering {
                Some(std::cmp::Ordering::Greater) => Some(AutoDecision::KeepFirst),
                Some(std::cmp::Ordering::Less) => Some(AutoDecision::KeepSecond),
                _ => None,
            };
        }

        // Non-side actions apply when the comparator is satisfied. `always`
        // and equal metrics match too, which is what "mark all pairs the same"
        // rules expect.
        if self.comparator == "always" || ordering.is_some() {
            return match self.action.as_str() {
                "same" => Some(AutoDecision::Same),
                "not_duplicates" => Some(AutoDecision::NotDuplicates),
                "delete_both" => Some(AutoDecision::DeleteBoth),
                _ => None,
            };
        }

        None
    }
}

pub fn compare(
    controller: &Controller,
    comparator: &str,
    a: &[u8],
    b: &[u8],
) -> Option<std::cmp::Ordering> {
    match comparator {
        "always" => Some(std::cmp::Ordering::Greater),
        "higher_resolution" => {
            measure(controller, a, resolution).partial_cmp(&measure(controller, b, resolution))
        }
        "larger_file" => measure(controller, a, size).partial_cmp(&measure(controller, b, size)),
        "more_tags" | "fewer_tags" => {
            let left = measure(controller, a, tag_count);
            let right = measure(controller, b, tag_count);
            let ordering = left.partial_cmp(&right);

            if comparator == "more_tags" {
                ordering
            } else {
                ordering.map(std::cmp::Ordering::reverse)
            }
        }
        "older" => {
            let left = measure(controller, a, archived_time);
            let right = measure(controller, b, archived_time);

            // A larger timestamp is older. Missing timestamps lose.
            match (left, right) {
                (Some(left), Some(right)) => right.partial_cmp(&left),
                (Some(_), None) => Some(std::cmp::Ordering::Greater),
                (None, Some(_)) => Some(std::cmp::Ordering::Less),
                (None, None) => None,
            }
        }
        _ => None,
    }
}

fn measure(
    controller: &Controller,
    hash: &[u8],
    metric: fn(&Controller, &[u8]) -> Option<f64>,
) -> Option<f64> {
    metric(controller, hash)
}

fn resolution(controller: &Controller, hash: &[u8]) -> Option<f64> {
    let info = super::query::file_info_for(controller, hash)?;

    Some(info.width? as f64 * info.height? as f64)
}

fn size(controller: &Controller, hash: &[u8]) -> Option<f64> {
    Some(super::query::file_info_for(controller, hash)?.size as f64)
}

fn tag_count(controller: &Controller, hash: &[u8]) -> Option<f64> {
    Some(query::selection_tags(controller, hash).len() as f64)
}

fn archived_time(controller: &Controller, hash: &[u8]) -> Option<f64> {
    let hash_id = query::hash_id_for(controller, hash)?;
    let timestamp = files_timestamps::get_file_timestamp(
        controller.db().db(),
        hash_id,
        TimestampType::Archived,
    )
    .ok()
    .flatten();

    timestamp.map(|value| value as f64)
}

pub fn load_rules(path: &Path) -> Vec<AutoResolutionRule> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_rules(path: &Path, rules: &[AutoResolutionRule]) -> std::io::Result<()> {
    let dump = serde_json::to_string_pretty(rules).map_err(std::io::Error::other)?;
    std::fs::write(path, dump)
}

pub fn rules_json(rules: &[AutoResolutionRule]) -> String {
    serde_json::json!({
        "rules": rules,
        "comparators": [
            {"value":"higher_resolution","label":"higher resolution"},
            {"value":"larger_file","label":"larger file"},
            {"value":"more_tags","label":"more tags"},
            {"value":"fewer_tags","label":"fewer tags"},
            {"value":"older","label":"older"},
            {"value":"always","label":"always"}
        ],
        "actions": [
            {"value":"keep_better","label":"keep the better file"},
            {"value":"same","label":"they are the same"},
            {"value":"not_duplicates","label":"they are not duplicates"},
            {"value":"delete_both","label":"delete both"}
        ]
    })
    .to_string()
}

pub fn dialog_json(state: &super::AppState) -> String {
    let mut value: serde_json::Value =
        serde_json::from_str(&rules_json(&state.auto_rules)).unwrap_or_default();
    value["enabled"] = serde_json::json!(
        state
            .controller
            .options()
            .get_boolean("duplicates_auto_resolution_enabled")
    );

    value.to_string()
}

pub fn add_rule(state: &mut super::AppState, payload: &str) -> Vec<super::UiCommand> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
        return vec![super::UiCommand::NotifyMessage(
            "invalid auto-resolution rule".to_string(),
        )];
    };

    let rule = AutoResolutionRule {
        name: value
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("auto-resolution rule")
            .to_string(),
        comparator: value
            .get("comparator")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("higher_resolution")
            .to_string(),
        action: value
            .get("action")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("keep_better")
            .to_string(),
    };

    state
        .auto_rules
        .retain(|existing| existing.name != rule.name);
    state.auto_rules.push(rule.clone());
    let _ = persist(state);
    state.message = format!("auto-resolution rule '{}' saved", rule.name);
    refresh(state)
}

pub fn remove_rule(state: &mut super::AppState, payload: &str) -> Vec<super::UiCommand> {
    let name = payload.trim();
    state.auto_rules.retain(|rule| rule.name != name);
    let _ = persist(state);
    state.message = format!("auto-resolution rule '{name}' removed");
    refresh(state)
}

pub fn persist(state: &super::AppState) -> std::io::Result<()> {
    save_rules(
        &state
            .controller
            .db_dir()
            .join("duplicate auto-resolution rules.json"),
        &state.auto_rules,
    )
}

fn refresh(state: &mut super::AppState) -> Vec<super::UiCommand> {
    state.refresh_all();

    let mut commands = state.refresh_commands();
    commands.push(super::UiCommand::DuplicatesRulesDialog(dialog_json(state)));
    commands.push(super::UiCommand::BuildPages(super::render::pages_json(
        state,
    )));
    commands.push(super::UiCommand::Status(super::render::status_json(state)));

    commands
}

#[cfg(test)]
mod tests;
