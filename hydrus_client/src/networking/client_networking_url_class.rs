//! URL class matching from `hydrus/client/networking/ClientNetworkingURLClass.py`.

use std::collections::BTreeMap;

use crate::networking::client_networking_functions::{ParsedUrl, parse_url};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathComponent {
    Fixed(String),
    Variable { name: String, default: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlClass {
    pub name: String,
    pub url_type: i32,
    pub preferred_scheme: Option<String>,
    pub netloc: String,
    pub path_components: Vec<PathComponent>,
    pub parameters: BTreeMap<String, Option<String>>,
    pub can_produce_multiple_files: bool,
    pub should_be_associated_with_files: bool,
}

impl UrlClass {
    pub fn new(name: impl Into<String>, netloc: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url_type: 0,
            preferred_scheme: Some("https".to_string()),
            netloc: netloc.into(),
            path_components: Vec::new(),
            parameters: BTreeMap::new(),
            can_produce_multiple_files: false,
            should_be_associated_with_files: true,
        }
    }

    pub fn with_fixed_path(
        mut self,
        components: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.path_components = components
            .into_iter()
            .map(|component| PathComponent::Fixed(component.into()))
            .collect();
        self
    }

    pub fn with_variable(mut self, name: impl Into<String>) -> Self {
        self.path_components.push(PathComponent::Variable {
            name: name.into(),
            default: String::new(),
        });
        self
    }

    pub fn matches(&self, url: &str) -> bool {
        self.parse(url).is_some()
    }

    /// Returns the extracted variables plus the parsed URL.
    pub fn parse(&self, url: &str) -> Option<(ParsedUrl, BTreeMap<String, String>)> {
        let parsed = parse_url(url)?;

        if parsed.netloc != self.netloc.to_lowercase() {
            return None;
        }

        if let Some(preferred_scheme) = &self.preferred_scheme {
            if parsed.scheme != preferred_scheme.to_lowercase()
                && parsed.scheme != "http"
                && parsed.scheme != "https"
            {
                return None;
            }
        }

        let path = parsed.path.trim_matches('/');
        let segments: Vec<&str> = if path.is_empty() {
            Vec::new()
        } else {
            path.split('/').collect()
        };

        if segments.len() != self.path_components.len() {
            return None;
        }

        let mut variables = BTreeMap::new();

        for (component, segment) in self.path_components.iter().zip(segments.iter()) {
            match component {
                PathComponent::Fixed(fixed) => {
                    if fixed != segment {
                        return None;
                    }
                }
                PathComponent::Variable { name, .. } => {
                    variables.insert(name.clone(), (*segment).to_string());
                }
            }
        }

        let parameters = parse_query(parsed.query.as_deref());

        for (name, expected) in &self.parameters {
            match (parameters.get(name), expected) {
                (Some(actual), Some(expected)) if actual == expected => {}
                (Some(_), None) => {}
                _ => return None,
            }
        }

        Some((parsed, variables))
    }

    pub fn normalise(&self, url: &str) -> Option<String> {
        let (parsed, _) = self.parse(url)?;
        let scheme = self
            .preferred_scheme
            .clone()
            .unwrap_or_else(|| parsed.scheme.clone());

        let path = parsed.path.trim_end_matches('/');

        let mut result = format!("{scheme}://{}{path}", self.netloc.to_lowercase());

        if let Some(query) = &parsed.query {
            let mut pairs: Vec<String> = query
                .split('&')
                .filter(|pair| !pair.is_empty())
                .map(str::to_string)
                .collect();
            pairs.sort();

            if !pairs.is_empty() {
                result.push('?');
                result.push_str(&pairs.join("&"));
            }
        }

        Some(result)
    }
}

fn parse_query(query: Option<&str>) -> BTreeMap<String, String> {
    let mut parameters = BTreeMap::new();

    if let Some(query) = query {
        for pair in query.split('&') {
            if let Some((name, value)) = pair.split_once('=') {
                parameters.insert(name.to_string(), value.to_string());
            }
        }
    }

    parameters
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url_class() -> UrlClass {
        UrlClass::new("post", "example.com")
            .with_fixed_path(["post"])
            .with_variable("post_id")
    }

    #[test]
    fn fixed_netloc_and_path_must_match() {
        let class = url_class();

        assert!(class.matches("https://example.com/post/123"));
        assert!(!class.matches("https://other.com/post/123"));
        assert!(!class.matches("https://example.com/other/123"));
    }

    #[test]
    fn variables_are_extracted() {
        let (_, variables) = url_class().parse("https://example.com/post/123").unwrap();

        assert_eq!(variables["post_id"], "123");
    }

    #[test]
    fn parameters_can_be_required() {
        let mut class = UrlClass::new("search", "example.com").with_fixed_path(["search"]);
        class.parameters.insert("q".to_string(), None);

        assert!(class.matches("https://example.com/search?q=blue"));
        assert!(!class.matches("https://example.com/search"));
    }

    #[test]
    fn normalisation_applies_scheme_and_sorts_parameters() {
        let class = UrlClass::new("search", "example.com").with_fixed_path(["search"]);

        assert_eq!(
            class
                .normalise("http://example.com/search?b=2&a=1")
                .unwrap(),
            "https://example.com/search?a=1&b=2"
        );
    }

    #[test]
    fn non_http_schemes_are_rejected() {
        assert!(!url_class().matches("ftp://example.com/post/1"));
    }
}
