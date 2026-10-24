//! Bandwidth manager from `hydrus/client/networking/ClientNetworkingBandwidth.py`.
//!
//! The rule/tracker arithmetic lives in `hydrus_core`; this module adds the
//! per-network-context bookkeeping the client needs.

use std::collections::BTreeMap;

use hydrus_core::hydrus_constants::BandwidthType;
use hydrus_core::networking::hydrus_networking::{BandwidthRules, BandwidthTracker};

use crate::networking::NetworkContext;

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
        self.rules
            .get(&context.to_string())
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
        let rules = self.rules.get(&key).unwrap_or(&self.default_rules);
        let tracker = self
            .trackers
            .entry(key)
            .or_insert_with(BandwidthTracker::new);

        rules.can_start_request(tracker, threshold)
    }

    pub fn can_continue_download(&mut self, context: &NetworkContext, threshold: i64) -> bool {
        let key = context.to_string();
        let rules = self.rules.get(&key).unwrap_or(&self.default_rules);
        let tracker = self
            .trackers
            .entry(key)
            .or_insert_with(BandwidthTracker::new);

        rules.can_continue_download(tracker, threshold)
    }

    pub fn get_waiting_estimate(&mut self, context: &NetworkContext) -> i64 {
        let key = context.to_string();
        let rules = self.rules.get(&key).unwrap_or(&self.default_rules);
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
}
