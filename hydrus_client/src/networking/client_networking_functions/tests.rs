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
    assert_eq!(convert_domain_into_top_level_domain("example.com"), "com");
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
    assert_eq!(
        convert_https_to_http("https://example.com"),
        "http://example.com"
    );
    assert_eq!(
        convert_http_to_https("http://example.com"),
        "https://example.com"
    );
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

#[test]
fn encoding_matches_python_examples() {
    assert_eq!(
        ensure_url_is_encoded("https://testbooru.cx/post/page.php?id=1234 56&s=view", true),
        "https://testbooru.cx/post/page.php?id=1234%2056&s=view"
    );
    assert_eq!(
        ensure_url_is_encoded(
            "https://testbooru.cx/post/page.php?id=1234%2056&s=view",
            true
        ),
        "https://testbooru.cx/post/page.php?id=1234%2056&s=view"
    );
    assert_eq!(
        ensure_url_is_encoded("https://weouthere.site/post?yo=1&name[id]=wew", true),
        "https://weouthere.site/post?yo=1&name%5Bid%5D=wew"
    );
    assert_eq!(
        ensure_url_is_encoded(
            "https://testbooru.cx/po@s%20t/page.php?id=1234 56&s=view%%25",
            true
        ),
        "https://testbooru.cx/po@s%20t/page.php?id=1234%2056&s=view%25%25"
    );
}

#[test]
fn fragment_is_kept_or_dropped_as_requested() {
    let url = "https://testbooru.cx/post/page.php?id=1234 56#hello";

    assert_eq!(
        ensure_url_is_encoded(url, true),
        "https://testbooru.cx/post/page.php?id=1234%2056#hello"
    );
    assert_eq!(
        ensure_url_is_encoded(url, false),
        "https://testbooru.cx/post/page.php?id=1234%2056"
    );
}
