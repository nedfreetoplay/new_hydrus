//! Per-domain download pools.
//!
//! Hydrus limits how many simultaneous connections each domain receives. The
//! GUI keeps explicit per-domain overrides here and uses the network options for
//! the global and default values. The worker uses [`DownloadPoolSettings::limit_for`]
//! to size each domain's pool.

use std::collections::BTreeMap;

use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadPoolSettings {
    /// Domain (without `www.`) to maximum concurrent downloads.
    pub domain_limits: BTreeMap<String, usize>,
}

impl DownloadPoolSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_limit(&mut self, domain: &str, limit: usize) {
        let domain = normalize_domain(domain);

        if domain.is_empty() {
            return;
        }

        if limit == 0 {
            self.domain_limits.remove(&domain);
        } else {
            self.domain_limits.insert(domain, limit.clamp(1, 64));
        }
    }

    pub fn remove_limit(&mut self, domain: &str) {
        self.domain_limits.remove(&normalize_domain(domain));
    }

    pub fn limit_for(&self, domain: &str, default_limit: usize) -> usize {
        self.domain_limits
            .get(&normalize_domain(domain))
            .copied()
            .unwrap_or(default_limit)
            .clamp(1, 64)
    }

    pub fn is_empty(&self) -> bool {
        self.domain_limits.is_empty()
    }

    /// Normalises keys loaded from an older or hand-edited settings file.
    pub fn normalize(&mut self) {
        let old = std::mem::take(&mut self.domain_limits);

        for (domain, limit) in old {
            self.set_limit(&domain, limit);
        }
    }
}

pub fn normalize_domain(domain: &str) -> String {
    let domain = domain
        .trim()
        .trim_start_matches("www.")
        .to_ascii_lowercase();

    if domain.is_empty() {
        return domain;
    }

    Url::parse(&format!("https://{domain}"))
        .ok()
        .and_then(|url| {
            url.host_str()
                .map(|host| host.trim_start_matches("www.").to_string())
        })
        .unwrap_or(domain)
}

pub fn domain_of(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|url| {
            url.host_str()
                .map(|host| host.trim_start_matches("www.").to_ascii_lowercase())
        })
        .unwrap_or_default()
}

/// Groups URLs by their domain while preserving the input order inside each
/// group. `BTreeMap` keeps the scheduling deterministic in tests.
pub fn group_by_domain(urls: Vec<String>) -> BTreeMap<String, Vec<String>> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for url in urls {
        let domain = domain_of(&url);
        groups.entry(domain).or_default().push(url);
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains_are_normalised() {
        assert_eq!(normalize_domain("www.Example.COM"), "example.com");
        assert_eq!(
            domain_of("https://cdn.example.com/a.jpg"),
            "cdn.example.com"
        );
        assert_eq!(domain_of("not a url"), "");
    }

    #[test]
    fn explicit_limits_override_the_default() {
        let mut settings = DownloadPoolSettings::new();
        settings.set_limit("example.com", 4);

        assert_eq!(settings.limit_for("www.example.com", 2), 4);
        assert_eq!(settings.limit_for("other.example", 2), 2);

        settings.remove_limit("example.com");

        assert_eq!(settings.limit_for("example.com", 2), 2);
    }

    #[test]
    fn urls_are_grouped_by_domain_in_order() {
        let groups = group_by_domain(vec![
            "https://a.example/1.jpg".to_string(),
            "https://b.example/2.jpg".to_string(),
            "https://a.example/3.jpg".to_string(),
        ]);

        assert_eq!(
            groups["a.example"],
            vec!["https://a.example/1.jpg", "https://a.example/3.jpg"]
        );
        assert_eq!(groups["b.example"], vec!["https://b.example/2.jpg"]);
    }
}
