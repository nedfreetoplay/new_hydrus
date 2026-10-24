
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
