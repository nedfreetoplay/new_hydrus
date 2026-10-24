//! URL helpers from `hydrus/client/networking/ClientNetworkingFunctions.py`.

use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};

const URL_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'{')
    .add(b'}');

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUrl {
    pub scheme: String,
    pub netloc: String,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
}

impl ParsedUrl {
    pub fn to_string(&self) -> String {
        let mut url = format!("{}://{}{}", self.scheme, self.netloc, self.path);

        if let Some(query) = &self.query {
            url.push('?');
            url.push_str(query);
        }

        if let Some(fragment) = &self.fragment {
            url.push('#');
            url.push_str(fragment);
        }

        url
    }
}

/// `ParseURL`
pub fn parse_url(url: &str) -> Option<ParsedUrl> {
    let (scheme, rest) = url.split_once("://")?;

    let (rest, fragment) = match rest.split_once('#') {
        Some((rest, fragment)) => (rest, Some(fragment.to_string())),
        None => (rest, None),
    };

    let (rest, query) = match rest.split_once('?') {
        Some((rest, query)) => (rest, Some(query.to_string())),
        None => (rest, None),
    };

    let (netloc, path) = match rest.split_once('/') {
        Some((netloc, path)) => (netloc, format!("/{path}")),
        None => (rest, "/".to_string()),
    };

    Some(ParsedUrl {
        scheme: scheme.to_lowercase(),
        netloc: netloc.to_lowercase(),
        path,
        query,
        fragment,
    })
}

/// `LooksLikeAFullURL`
pub fn looks_like_a_full_url(text: &str) -> bool {
    parse_url(text.trim()).is_some()
}

/// `ConvertURLIntoDomain`
pub fn convert_url_into_domain(url: &str) -> String {
    parse_url(url)
        .map(|parsed| parsed.netloc)
        .unwrap_or_else(|| url.trim().to_lowercase())
}

pub fn remove_www_from_domain(domain: &str) -> String {
    domain.strip_prefix("www.").unwrap_or(domain).to_string()
}

/// `ConvertDomainIntoSecondLevelDomain`
pub fn convert_domain_into_second_level_domain(domain: &str) -> String {
    let domain = remove_www_from_domain(&domain.to_lowercase());
    let labels: Vec<&str> = domain.split('.').filter(|label| !label.is_empty()).collect();

    if labels.len() <= 2 {
        labels.join(".")
    } else {
        labels[labels.len() - 2..].join(".")
    }
}

/// `ConvertDomainIntoTopLevelDomain`
pub fn convert_domain_into_top_level_domain(domain: &str) -> String {
    let domain = remove_www_from_domain(&domain.to_lowercase());

    domain
        .rsplit('.')
        .next()
        .unwrap_or(domain.as_str())
        .to_string()
}

/// `ConvertDomainIntoNextLevelDomain`
pub fn convert_domain_into_next_level_domain(domain: &str) -> Option<String> {
    let domain = remove_www_from_domain(&domain.to_lowercase());

    domain.split_once('.').map(|(_, rest)| rest.to_string())
}

/// `ConvertDomainIntoAllApplicableDomains`
pub fn convert_domain_into_all_applicable_domains(domain: &str, discard_www: bool) -> Vec<String> {
    let mut domain = domain.to_lowercase();

    if discard_www {
        domain = remove_www_from_domain(&domain);
    }

    let labels: Vec<&str> = domain.split('.').filter(|label| !label.is_empty()).collect();
    let mut domains = Vec::new();

    for index in 0..labels.len().saturating_sub(1) {
        domains.push(labels[index..].join("."));
    }

    if domains.is_empty() && !domain.is_empty() {
        domains.push(domain);
    }

    domains
}

/// `ConvertHTTPSToHTTP` / `ConvertHTTPToHTTPS`
pub fn convert_https_to_http(url: &str) -> String {
    url.strip_prefix("https://")
        .map(|rest| format!("http://{rest}"))
        .unwrap_or_else(|| url.to_string())
}

pub fn convert_http_to_https(url: &str) -> String {
    url.strip_prefix("http://")
        .map(|rest| format!("https://{rest}"))
        .unwrap_or_else(|| url.to_string())
}

/// `EnsureURLIsEncoded`
pub fn ensure_url_is_encoded(url: &str, keep_fragment: bool) -> String {
    let (url, fragment) = if keep_fragment {
        (url, None)
    } else {
        match url.split_once('#') {
            Some((url, fragment)) => (url, Some(fragment.to_string())),
            None => (url, None),
        }
    };

    let encoded = utf8_percent_encode(url, URL_ENCODE_SET).to_string();

    match fragment {
        Some(fragment) => format!("{encoded}#{fragment}"),
        None => encoded,
    }
}

/// `NormaliseAndFilterAssociableURLs`
pub fn normalise_and_filter_associable_urls(
    urls: impl IntoIterator<Item = String>,
) -> Vec<String> {
    let mut result = Vec::new();

    for url in urls {
        let url = ensure_url_is_encoded(url.trim(), true);

        if !url.is_empty() && !result.contains(&url) {
            result.push(url);
        }
    }

    result
}

/// `ConvertURLToHumanString`
pub fn convert_url_to_human_string(url: &str) -> String {
    percent_decode_str(url)
        .decode_utf8_lossy()
        .replace("https://", "")
        .replace("http://", "")
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_parse_into_components() {
        let parsed = parse_url("https://Example.com/a/b?x=1#frag").unwrap();

        assert_eq!(parsed.scheme, "https");
        assert_eq!(parsed.netloc, "example.com");
        assert_eq!(parsed.path, "/a/b");
        assert_eq!(parsed.query.as_deref(), Some("x=1"));
        assert_eq!(parsed.fragment.as_deref(), Some("frag"));
        assert_eq!(parsed.to_string(), "https://example.com/a/b?x=1#frag");
    }

    #[test]
    fn url_detection_requires_a_scheme() {
        assert!(looks_like_a_full_url("https://example.com"));
        assert!(!looks_like_a_full_url("example.com"));
    }

    #[test]
    fn domains_reduce_to_second_level() {
        assert_eq!(
            convert_domain_into_second_level_domain("www.a.example.co.uk"),
            "co.uk"
        );
        assert_eq!(
            convert_domain_into_second_level_domain("example.com"),
            "example.com"
        );
        assert_eq!(
            convert_domain_into_top_level_domain("example.com"),
            "com"
        );
    }

    #[test]
    fn applicable_domains_walk_up_the_labels() {
        assert_eq!(
            convert_domain_into_all_applicable_domains("a.b.example.com", true),
            vec!["a.b.example.com", "b.example.com", "example.com"]
        );
    }

    #[test]
    fn scheme_conversion_round_trips() {
        assert_eq!(convert_https_to_http("https://example.com"), "http://example.com");
        assert_eq!(convert_http_to_https("http://example.com"), "https://example.com");
    }

    #[test]
    fn urls_are_encoded_and_deduped() {
        let urls = normalise_and_filter_associable_urls(vec![
            "https://example.com/a b".to_string(),
            " https://example.com/a%20b ".to_string(),
            String::new(),
        ]);

        assert_eq!(urls, vec!["https://example.com/a%20b".to_string()]);
    }

    #[test]
    fn human_string_strips_scheme_and_slash() {
        assert_eq!(
            convert_url_to_human_string("https://example.com/a%20b/"),
            "example.com/a b"
        );
    }
}
