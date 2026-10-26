//! Network defaults from `ClientDefaults.py`.

use std::collections::BTreeMap;

use hydrus_core::hydrus_constants::BandwidthType;
use hydrus_core::networking::hydrus_networking::BandwidthRules;
use serde::{Deserialize, Serialize};

use crate::client_constants::NetworkContextType;
use crate::networking::NetworkContext;
use crate::networking::client_networking_bandwidth::NetworkBandwidthManager;

pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (compatible; Hydrus Client)";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomHeader {
    pub value: String,
    pub approved: bool,
    pub note: String,
}

pub fn default_custom_headers() -> BTreeMap<String, CustomHeader> {
    let mut headers = BTreeMap::new();

    headers.insert(
        "User-Agent".to_string(),
        CustomHeader {
            value: DEFAULT_USER_AGENT.to_string(),
            approved: true,
            note: "This is the default User-Agent identifier for the client for all network connections."
                .to_string(),
        },
    );
    headers.insert(
        "Accept".to_string(),
        CustomHeader {
            value: "image/jpeg,image/png,image/*;q=0.9,*/*;q=0.8".to_string(),
            approved: true,
            note: "Prefers jpeg/png over webp, but provides graceful fallback.".to_string(),
        },
    );
    headers.insert(
        "Cache-Control".to_string(),
        CustomHeader {
            value: "no-transform".to_string(),
            approved: true,
            note: "Tells CDNs not to deliver \"optimised\" versions of files.".to_string(),
        },
    );

    headers
}

fn rules(entries: &[(BandwidthType, i64, i64)]) -> BandwidthRules {
    let rules = BandwidthRules::new();

    for (bandwidth_type, time_delta, max_allowed) in entries {
        rules.add_rule(*bandwidth_type, Some(*time_delta), *max_allowed);
    }

    rules
}

/// `set_default_bandwidth_manager_rules`
pub fn apply_default_bandwidth_rules(manager: &mut NetworkBandwidthManager) {
    const KB: i64 = 1024;
    const MB: i64 = KB * KB;
    const GB: i64 = MB * 1024;

    manager.set_rules(
        &NetworkContext::default_context(),
        rules(&[
            (BandwidthType::Requests, 1, 5),
            (BandwidthType::Data, 86_400, 16 * GB),
        ]),
    );

    manager.set_rules(
        &NetworkContext::new(NetworkContextType::Domain, None),
        rules(&[
            (BandwidthType::Requests, 1, 1),
            (BandwidthType::Data, 86_400, 8 * GB),
        ]),
    );

    manager.set_rules(
        &NetworkContext::hydrus(),
        rules(&[(BandwidthType::Data, 86_400, 2 * GB)]),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_headers_match_python_defaults() {
        let headers = default_custom_headers();

        assert_eq!(headers["User-Agent"].value, DEFAULT_USER_AGENT);
        assert_eq!(headers["Accept"].approved, true);
        assert!(headers.contains_key("Cache-Control"));
    }

    #[test]
    fn bandwidth_defaults_are_installed_for_three_contexts() {
        let mut manager = NetworkBandwidthManager::new();

        apply_default_bandwidth_rules(&mut manager);

        assert!(manager.has_rules(&NetworkContext::default_context()));
        assert!(manager.has_rules(&NetworkContext::hydrus()));
        assert!(manager.has_rules(&NetworkContext::new(NetworkContextType::Domain, None)));
    }

    #[test]
    fn domain_rule_blocks_rapid_requests() {
        let mut manager = NetworkBandwidthManager::new();
        apply_default_bandwidth_rules(&mut manager);
        let context = NetworkContext::for_domain("example.com");

        assert!(manager.can_start_request(&context, 0));

        manager.report_request_used(&context, 1);

        assert!(!manager.can_start_request(&context, 0));
    }
}
