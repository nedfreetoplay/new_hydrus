//! URL class matching from `hydrus/client/networking/ClientNetworkingURLClass.py`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::networking::client_networking_functions::{ParsedUrl, parse_url};

pub const URL_TYPE_POST: i32 = 0;
pub const URL_TYPE_API: i32 = 1;
pub const URL_TYPE_FILE: i32 = 2;
pub const URL_TYPE_GALLERY: i32 = 3;
pub const URL_TYPE_WATCHABLE: i32 = 4;
pub const URL_TYPE_UNKNOWN: i32 = 5;
pub const URL_TYPE_NEXT: i32 = 6;
pub const URL_TYPE_DESIRED: i32 = 7;
pub const URL_TYPE_SOURCE: i32 = 8;
pub const URL_TYPE_SUB_GALLERY: i32 = 9;

pub fn url_type_name(value: i32) -> &'static str {
    match value {
        URL_TYPE_POST => "post url",
        URL_TYPE_API => "api/redirect url",
        URL_TYPE_FILE => "file url",
        URL_TYPE_GALLERY => "gallery url",
        URL_TYPE_WATCHABLE => "watchable url",
        URL_TYPE_UNKNOWN => "unknown url",
        URL_TYPE_NEXT => "next page url",
        URL_TYPE_DESIRED => "downloadable/pursuable url",
        URL_TYPE_SOURCE => "source url",
        URL_TYPE_SUB_GALLERY => "sub-gallery url",
        _ => "unknown url",
    }
}

pub fn url_type_from_name(value: &str) -> Option<i32> {
    match value.trim().to_ascii_lowercase().as_str() {
        "post" | "post url" => Some(URL_TYPE_POST),
        "api" | "api/redirect" | "api/redirect url" => Some(URL_TYPE_API),
        "file" | "file url" => Some(URL_TYPE_FILE),
        "gallery" | "gallery url" => Some(URL_TYPE_GALLERY),
        "watchable" | "watchable url" => Some(URL_TYPE_WATCHABLE),
        "unknown" | "unknown url" => Some(URL_TYPE_UNKNOWN),
        "next" | "next page url" => Some(URL_TYPE_NEXT),
        "desired" | "downloadable/pursuable url" => Some(URL_TYPE_DESIRED),
        "source" | "source url" => Some(URL_TYPE_SOURCE),
        "sub-gallery" | "sub-gallery url" => Some(URL_TYPE_SUB_GALLERY),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathComponent {
    Fixed(String),
    Variable { name: String, default: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlClass {
    pub name: String,
    #[serde(default)]
    pub url_type: i32,
    #[serde(default)]
    pub preferred_scheme: Option<String>,
    pub netloc: String,
    #[serde(default)]
    pub path_components: Vec<PathComponent>,
    #[serde(default)]
    pub parameters: BTreeMap<String, Option<String>>,
    #[serde(default)]
    pub can_produce_multiple_files: bool,
    #[serde(default = "default_true")]
    pub should_be_associated_with_files: bool,
    /// Match any subdomain of `netloc` (`cdn.example.com` for `example.com`).
    #[serde(default)]
    pub match_subdomains: bool,
    /// Keep the matched subdomain when normalising instead of replacing it with
    /// `netloc`.
    #[serde(default)]
    pub keep_matched_subdomains: bool,
    #[serde(default)]
    pub example_url: String,
}

fn default_true() -> bool {
    true
}

impl UrlClass {
    pub fn new(name: impl Into<String>, netloc: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url_type: URL_TYPE_POST,
            preferred_scheme: Some("https".to_string()),
            netloc: netloc.into(),
            path_components: Vec::new(),
            parameters: BTreeMap::new(),
            can_produce_multiple_files: false,
            should_be_associated_with_files: true,
            match_subdomains: false,
            keep_matched_subdomains: false,
            example_url: String::new(),
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

        let netloc = self.netloc.to_ascii_lowercase();
        let host = parsed.netloc.to_ascii_lowercase();

        let host_matches = if self.match_subdomains {
            host == netloc || host.ends_with(&format!(".{netloc}"))
        } else {
            host == netloc
        };

        if !host_matches {
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

        let netloc = if self.keep_matched_subdomains {
            parsed.netloc.to_ascii_lowercase()
        } else {
            self.netloc.to_ascii_lowercase()
        };

        let mut result = format!("{scheme}://{netloc}{path}");

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
mod tests;
