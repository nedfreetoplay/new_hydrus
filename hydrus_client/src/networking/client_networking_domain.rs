//! Domain metadata and URL-class selection from `ClientNetworkingDomain.py`.

use std::collections::BTreeMap;

use crate::client_defaults::CustomHeader;
use crate::networking::client_networking_url_class::UrlClass;

#[derive(Debug, Default)]
pub struct DomainManager {
    url_classes: Vec<UrlClass>,
    custom_headers: BTreeMap<String, CustomHeader>,
}

impl DomainManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_url_class(&mut self, url_class: UrlClass) {
        self.url_classes.push(url_class);
    }

    pub fn get_url_classes(&self) -> &[UrlClass] {
        &self.url_classes
    }

    pub fn get_url_class_for_url(&self, url: &str) -> Option<&UrlClass> {
        self.url_classes
            .iter()
            .find(|url_class| url_class.matches(url))
    }

    pub fn normalise_url(&self, url: &str) -> Option<String> {
        self.get_url_class_for_url(url)?.normalise(url)
    }

    pub fn set_custom_header(&mut self, name: impl Into<String>, header: CustomHeader) {
        self.custom_headers.insert(name.into(), header);
    }

    pub fn get_custom_header(&self, name: &str) -> Option<&CustomHeader> {
        self.custom_headers.get(name)
    }

    pub fn remove_custom_header(&mut self, name: &str) -> bool {
        self.custom_headers.remove(name).is_some()
    }

    pub fn get_custom_headers(&self) -> &BTreeMap<String, CustomHeader> {
        &self.custom_headers
    }

    pub fn get_custom_header_pairs(&self) -> Vec<(String, String)> {
        self.custom_headers
            .iter()
            .filter(|(_, header)| header.approved)
            .map(|(name, header)| (name.clone(), header.value.clone()))
            .collect()
    }

    pub fn clear_url_classes(&mut self) {
        self.url_classes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::client_networking_url_class::UrlClass;

    fn manager() -> DomainManager {
        let mut manager = DomainManager::new();
        manager.add_url_class(
            UrlClass::new("post", "example.com")
                .with_fixed_path(["post"])
                .with_variable("post_id"),
        );

        manager
    }

    #[test]
    fn url_classes_are_matched_by_domain_and_path() {
        let manager = manager();

        assert!(manager.get_url_class_for_url("https://example.com/post/1").is_some());
        assert!(manager.get_url_class_for_url("https://example.com/nope/1").is_none());
    }

    #[test]
    fn normalisation_uses_the_matching_class() {
        let manager = manager();

        assert_eq!(
            manager.normalise_url("http://example.com/post/1").unwrap(),
            "https://example.com/post/1"
        );
    }

    #[test]
    fn custom_headers_round_trip_and_filter_approved() {
        let mut manager = DomainManager::new();
        manager.set_custom_header(
            "User-Agent",
            CustomHeader {
                value: "hydrus".to_string(),
                approved: true,
                note: String::new(),
            },
        );
        manager.set_custom_header(
            "Bad",
            CustomHeader {
                value: "no".to_string(),
                approved: false,
                note: String::new(),
            },
        );

        assert_eq!(manager.get_custom_header("User-Agent").unwrap().value, "hydrus");
        assert_eq!(
            manager.get_custom_header_pairs(),
            vec![("User-Agent".to_string(), "hydrus".to_string())]
        );
    }

    #[test]
    fn removing_headers_works() {
        let mut manager = DomainManager::new();
        manager.set_custom_header(
            "X",
            CustomHeader {
                value: "1".to_string(),
                approved: true,
                note: String::new(),
            },
        );

        assert!(manager.remove_custom_header("X"));
        assert!(!manager.remove_custom_header("X"));
    }
}
