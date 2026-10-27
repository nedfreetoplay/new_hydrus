//! Bandwidth manager from `hydrus/client/networking/ClientNetworkingBandwidth.py`.
//!
//! The rule/tracker arithmetic lives in `hydrus_core`; this module adds the
//! per-network-context bookkeeping the client needs.

use std::collections::BTreeMap;

use hydrus_core::hydrus_constants::BandwidthType;
use hydrus_core::networking::hydrus_networking::{BandwidthRules, BandwidthTracker};
use serde::{Deserialize, Serialize};

use crate::client_constants::NetworkContextType;
use crate::networking::{ContextData, NetworkContext};

#[derive(Default)]
pub struct NetworkBandwidthManager {
    default_rules: BandwidthRules,
    rules: BTreeMap<String, BandwidthRules>,
    trackers: BTreeMap<String, BandwidthTracker>,
}

impl NetworkBandwidthManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_rules(&mut self, context: &NetworkContext, rules: BandwidthRules) {
        self.rules.insert(context.to_string(), rules);
    }

    pub fn get_rules(&self, context: &NetworkContext) -> &BandwidthRules {
        self.rules_for(context)
    }

    fn rules_for(&self, context: &NetworkContext) -> &BandwidthRules {
        self.rules
            .get(&context.to_string())
            .or_else(|| {
                self.rules
                    .get(&NetworkContext::new(context.context_type, None).to_string())
            })
            .unwrap_or(&self.default_rules)
    }

    pub fn has_rules(&self, context: &NetworkContext) -> bool {
        self.rules.contains_key(&context.to_string())
    }

    pub fn delete_rules(&mut self, context: &NetworkContext) {
        self.rules.remove(&context.to_string());
    }

    pub fn get_default_rules(&self) -> &BandwidthRules {
        &self.default_rules
    }

    pub fn set_default_rules(&mut self, rules: BandwidthRules) {
        self.default_rules = rules;
    }

    pub fn get_tracker(&mut self, context: &NetworkContext) -> &BandwidthTracker {
        self.trackers
            .entry(context.to_string())
            .or_insert_with(BandwidthTracker::new)
    }

    pub fn can_start_request(&mut self, context: &NetworkContext, threshold: i64) -> bool {
        let key = context.to_string();
        let generic_key = NetworkContext::new(context.context_type, None).to_string();
        let rules = self
            .rules
            .get(&key)
            .or_else(|| self.rules.get(&generic_key))
            .unwrap_or(&self.default_rules);
        let tracker = self
            .trackers
            .entry(key)
            .or_insert_with(BandwidthTracker::new);

        rules.can_start_request(tracker, threshold)
    }

    pub fn can_continue_download(&mut self, context: &NetworkContext, threshold: i64) -> bool {
        let key = context.to_string();
        let generic_key = NetworkContext::new(context.context_type, None).to_string();
        let rules = self
            .rules
            .get(&key)
            .or_else(|| self.rules.get(&generic_key))
            .unwrap_or(&self.default_rules);
        let tracker = self
            .trackers
            .entry(key)
            .or_insert_with(BandwidthTracker::new);

        rules.can_continue_download(tracker, threshold)
    }

    pub fn get_waiting_estimate(&mut self, context: &NetworkContext) -> i64 {
        let key = context.to_string();
        let generic_key = NetworkContext::new(context.context_type, None).to_string();
        let rules = self
            .rules
            .get(&key)
            .or_else(|| self.rules.get(&generic_key))
            .unwrap_or(&self.default_rules);
        let tracker = self
            .trackers
            .entry(key)
            .or_insert_with(BandwidthTracker::new);

        rules.get_waiting_estimate(tracker)
    }

    pub fn report_data_used(&mut self, context: &NetworkContext, num_bytes: i64) {
        self.get_tracker(context).report_data_used(num_bytes);
    }

    pub fn report_request_used(&mut self, context: &NetworkContext, num_requests: i64) {
        self.get_tracker(context).report_request_used(num_requests);
    }

    pub fn get_current_month_summary(&self, context: &NetworkContext) -> Option<String> {
        self.trackers
            .get(&context.to_string())
            .map(BandwidthTracker::get_current_month_summary)
    }

    pub fn delete_history(&mut self, context: &NetworkContext) {
        self.trackers.remove(&context.to_string());
    }
}

pub fn rule_requests(time_delta: i64, max_allowed: i64) -> BandwidthRules {
    let rules = BandwidthRules::new();
    rules.add_rule(BandwidthType::Requests, Some(time_delta), max_allowed);
    rules
}

pub fn rule_data(time_delta: i64, max_allowed: i64) -> BandwidthRules {
    let rules = BandwidthRules::new();
    rules.add_rule(BandwidthType::Data, Some(time_delta), max_allowed);
    rules
}

/// Serializable form of one bandwidth rule. The GUI edits a flat table and the
/// controller rebuilds [`BandwidthRules`] for each context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BandwidthRuleEntry {
    /// `global`, `hydrus`, `domain`, ...
    pub context_type: String,
    /// Domain name for domain contexts, otherwise empty.
    pub context: String,
    /// `requests` or `data`.
    pub bandwidth_type: String,
    /// Seconds, or `None` for the current calendar month.
    pub time_delta: Option<i64>,
    pub max_allowed: i64,
}

impl Default for BandwidthRuleEntry {
    fn default() -> Self {
        Self {
            context_type: "domain".to_string(),
            context: String::new(),
            bandwidth_type: "requests".to_string(),
            time_delta: Some(1),
            max_allowed: 1,
        }
    }
}

impl NetworkBandwidthManager {
    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }

    /// Applies a flat list of UI rules on top of the current rules. Existing
    /// rules for every context mentioned in `entries` are replaced.
    pub fn apply_rule_entries(&mut self, entries: &[BandwidthRuleEntry]) {
        let mut grouped: BTreeMap<String, (NetworkContext, BandwidthRules)> = BTreeMap::new();

        for entry in entries {
            let context = context_from_entry(entry);
            let key = context.to_string();
            let (_, rules) = grouped
                .entry(key)
                .or_insert_with(|| (context, BandwidthRules::new()));

            let bandwidth_type = bandwidth_type_from_name(&entry.bandwidth_type);
            rules.add_rule(bandwidth_type, entry.time_delta, entry.max_allowed.max(0));
        }

        for (key, (context, rules)) in grouped {
            self.rules.remove(&key);
            self.set_rules(&context, rules);
        }
    }

    pub fn get_rule_contexts(&self) -> Vec<String> {
        self.rules.keys().cloned().collect()
    }
}

pub fn default_rule_entries() -> Vec<BandwidthRuleEntry> {
    vec![
        BandwidthRuleEntry {
            context_type: "global".to_string(),
            context: String::new(),
            bandwidth_type: "requests".to_string(),
            time_delta: Some(1),
            max_allowed: 5,
        },
        BandwidthRuleEntry {
            context_type: "global".to_string(),
            context: String::new(),
            bandwidth_type: "data".to_string(),
            time_delta: Some(86_400),
            max_allowed: 16 * 1024 * 1024 * 1024,
        },
        BandwidthRuleEntry {
            context_type: "domain".to_string(),
            context: String::new(),
            bandwidth_type: "requests".to_string(),
            time_delta: Some(1),
            max_allowed: 1,
        },
        BandwidthRuleEntry {
            context_type: "domain".to_string(),
            context: String::new(),
            bandwidth_type: "data".to_string(),
            time_delta: Some(86_400),
            max_allowed: 8 * 1024 * 1024 * 1024,
        },
        BandwidthRuleEntry {
            context_type: "hydrus".to_string(),
            context: String::new(),
            bandwidth_type: "data".to_string(),
            time_delta: Some(86_400),
            max_allowed: 2 * 1024 * 1024 * 1024,
        },
    ]
}

pub fn context_from_entry(entry: &BandwidthRuleEntry) -> NetworkContext {
    let context_type = context_type_from_name(&entry.context_type);

    match context_type {
        NetworkContextType::Global => NetworkContext::default_context(),
        NetworkContextType::Hydrus => NetworkContext::hydrus(),
        NetworkContextType::Domain if entry.context.trim().is_empty() => {
            NetworkContext::new(NetworkContextType::Domain, None)
        }
        NetworkContextType::Domain => NetworkContext::for_domain(entry.context.clone()),
        other => NetworkContext::new(
            other,
            Some(ContextData::Text(entry.context.clone())).filter(|_| !entry.context.is_empty()),
        ),
    }
}

pub fn context_type_from_name(value: &str) -> NetworkContextType {
    match value.trim().to_ascii_lowercase().as_str() {
        "global" => NetworkContextType::Global,
        "hydrus" => NetworkContextType::Hydrus,
        "downloader" => NetworkContextType::Downloader,
        "downloader page" | "downloader_page" => NetworkContextType::DownloaderPage,
        "subscription" => NetworkContextType::Subscription,
        "watcher page" | "watcher_page" => NetworkContextType::WatcherPage,
        _ => NetworkContextType::Domain,
    }
}

pub fn bandwidth_type_from_name(value: &str) -> BandwidthType {
    match value.trim().to_ascii_lowercase().as_str() {
        "data" | "bytes" => BandwidthType::Data,
        _ => BandwidthType::Requests,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::NetworkContext;

    #[test]
    fn request_rules_are_enforced_per_context() {
        let context = NetworkContext::for_domain("example.com");
        let mut manager = NetworkBandwidthManager::new();
        manager.set_rules(&context, rule_requests(60, 1));

        assert!(manager.can_start_request(&context, 0));

        manager.report_request_used(&context, 1);

        assert!(!manager.can_start_request(&context, 0));
        assert!(manager.get_waiting_estimate(&context) > 0);
    }

    #[test]
    fn contexts_do_not_share_trackers() {
        let first = NetworkContext::for_domain("a.com");
        let second = NetworkContext::for_domain("b.com");
        let mut manager = NetworkBandwidthManager::new();
        manager.set_rules(&first, rule_requests(60, 1));
        manager.set_rules(&second, rule_requests(60, 1));

        manager.report_request_used(&first, 1);

        assert!(!manager.can_start_request(&first, 0));
        assert!(manager.can_start_request(&second, 0));
    }

    #[test]
    fn deleting_rules_falls_back_to_defaults() {
        let context = NetworkContext::for_domain("example.com");
        let mut manager = NetworkBandwidthManager::new();
        manager.set_rules(&context, rule_requests(60, 1));

        assert!(manager.has_rules(&context));

        manager.delete_rules(&context);

        assert!(!manager.has_rules(&context));
        assert!(manager.can_start_request(&context, 0));
    }

    #[test]
    fn data_usage_is_tracked() {
        let context = NetworkContext::for_domain("example.com");
        let mut manager = NetworkBandwidthManager::new();
        manager.report_data_used(&context, 1024);

        assert!(manager.get_current_month_summary(&context).is_some());
    }

    #[test]
    fn empty_domain_context_targets_the_domain_default() {
        let context =
            NetworkContext::new(crate::client_constants::NetworkContextType::Domain, None);
        let mut manager = NetworkBandwidthManager::new();
        manager.apply_rule_entries(&[BandwidthRuleEntry {
            context_type: "domain".to_string(),
            context: String::new(),
            bandwidth_type: "requests".to_string(),
            time_delta: Some(60),
            max_allowed: 1,
        }]);

        assert!(manager.can_start_request(&NetworkContext::for_domain("a.com"), 0));
        manager.report_request_used(&NetworkContext::for_domain("a.com"), 1);
        assert!(!manager.can_start_request(&NetworkContext::for_domain("a.com"), 0));
        assert!(manager.has_rules(&context));
    }

    #[test]
    fn flat_rule_entries_replace_context_rules() {
        let context = NetworkContext::for_domain("example.com");
        let mut manager = NetworkBandwidthManager::new();
        manager.apply_rule_entries(&[BandwidthRuleEntry {
            context_type: "domain".to_string(),
            context: "example.com".to_string(),
            bandwidth_type: "requests".to_string(),
            time_delta: Some(60),
            max_allowed: 1,
        }]);

        assert!(manager.can_start_request(&context, 0));
        manager.report_request_used(&context, 1);
        assert!(!manager.can_start_request(&context, 0));

        manager.apply_rule_entries(&[BandwidthRuleEntry {
            context_type: "domain".to_string(),
            context: "example.com".to_string(),
            bandwidth_type: "requests".to_string(),
            time_delta: Some(60),
            max_allowed: 10,
        }]);
        manager.delete_history(&context);

        assert!(manager.can_start_request(&context, 0));
    }
}
