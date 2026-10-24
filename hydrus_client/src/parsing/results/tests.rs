use super::*;

#[test]
fn descriptions_render_names_and_kinds() {
    let description = ParsableContentDescription::new(
        "source",
        ContentKind::Url {
            url_type: 1,
            priority: 2,
        },
    );

    let text = description.to_string(Some("https://example.com"));

    assert!(text.contains("url"));
    assert!(text.contains("p2"));
    assert!(text.contains("https://example.com"));
}

#[test]
fn tags_keep_their_namespace() {
    let description = ParsableContentDescription::new(
        "creator",
        ContentKind::Tag {
            namespace: Some("creator".to_string()),
        },
    );

    assert_eq!(
        description
            .get_shorthand_content_specific_info_string()
            .as_deref(),
        Some("creator")
    );
}

#[test]
fn parsed_posts_collect_urls_and_tags() {
    let mut post = ParsedPost::new("https://example.com/post");

    post.add_content(ParsedContent::new(
        ParsableContentDescription::new(
            "image",
            ContentKind::Url {
                url_type: 0,
                priority: 0,
            },
        ),
        "https://example.com/image.jpg",
    ));
    post.add_content(ParsedContent::new(
        ParsableContentDescription::new("tag", ContentKind::Tag { namespace: None }),
        "blue eyes",
    ));
    post.add_tag("local tags", "blue eyes");

    assert_eq!(post.get_urls(), vec!["https://example.com/image.jpg"]);
    assert_eq!(post.tags["local tags"], vec!["blue eyes"]);
    assert!(!post.has_veto());
}

#[test]
fn veto_content_is_detected() {
    let mut post = ParsedPost::new("https://example.com/post");
    post.add_content(ParsedContent::new(
        ParsableContentDescription::new("stop", ContentKind::Veto),
        "1",
    ));

    assert!(post.has_veto());
}
