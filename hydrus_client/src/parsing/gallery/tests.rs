use super::*;

#[test]
fn danbooru_gallery_finds_posts_and_next_page() {
    let html = r#"
        <article data-id="1"><a href="/posts/101?q=blue">one</a></article>
        <article data-id="2"><a href="/posts/102">two</a></article>
        <a rel="next" href="/posts?tags=blue&page=2">next</a>
    "#;

    let parser = parser_for_url("https://danbooru.donmai.us/posts?tags=blue");
    let parsed = parse_gallery_page(html, "https://danbooru.donmai.us/posts?tags=blue", parser);

    assert_eq!(
        parsed.posts,
        vec![
            "https://danbooru.donmai.us/posts/101?q=blue".to_string(),
            "https://danbooru.donmai.us/posts/102".to_string()
        ]
    );
    assert_eq!(
        parsed.next_pages,
        vec!["https://danbooru.donmai.us/posts?tags=blue&page=2".to_string()]
    );
    assert!(!parsed.is_post);
}

#[test]
fn danbooru_post_uses_open_graph_image() {
    let html = r#"
        <meta property="og:image" content="https://cdn.donmai.us/original/aa/bb/aa-bb.jpg">
        <video><source src="https://cdn.donmai.us/original/aa/bb/aa-bb.webm"></video>
    "#;

    let parser = parser_for_url("https://danbooru.donmai.us/posts/101");
    let files = parse_post_page(html, "https://danbooru.donmai.us/posts/101", parser);

    assert!(files.contains(&"https://cdn.donmai.us/original/aa/bb/aa-bb.jpg".to_string()));
    assert!(files.contains(&"https://cdn.donmai.us/original/aa/bb/aa-bb.webm".to_string()));
}

#[test]
fn gelbooru_gallery_recognises_view_links() {
    let html = r#"
        <a href="index.php?page=post&amp;s=view&amp;id=123">one</a>
        <a href="index.php?page=post&amp;s=list&amp;tags=blue&amp;pid=42">next</a>
    "#;

    let parser = parser_for_url("https://gelbooru.com/index.php?page=post&s=list&tags=blue");
    let parsed = parse_gallery_page(
        html,
        "https://gelbooru.com/index.php?page=post&s=list&tags=blue",
        parser,
    );

    assert_eq!(
        parsed.posts,
        vec!["https://gelbooru.com/index.php?page=post&s=view&id=123".to_string()]
    );
    assert_eq!(
        parsed.next_pages,
        vec!["https://gelbooru.com/index.php?page=post&s=list&tags=blue&pid=42".to_string()]
    );
}

#[test]
fn pixiv_post_reads_embedded_original_url() {
    let html = r#"<script>window.__data = {"original":"https://i.pximg.net/img-original/img/2024/01/01/00/00/00/123_p0.jpg"};</script>"#;

    let parser = parser_by_id("pixiv");
    let files = parse_post_page(html, "https://www.pixiv.net/artworks/123", parser);

    assert!(files.contains(
        &"https://i.pximg.net/img-original/img/2024/01/01/00/00/00/123_p0.jpg".to_string()
    ));
}

#[test]
fn generic_gallery_falls_back_to_page_links() {
    let html = r#"
        <a href="/post/1">one</a>
        <img src="/thumb/1.jpg">
        <a href="https://other.example/post/2">offsite</a>
    "#;

    let parser = parser_by_id("generic");
    let parsed = parse_gallery_page(html, "https://example.com/gallery", parser);

    assert!(
        parsed
            .files
            .contains(&"https://example.com/thumb/1.jpg".to_string())
    );
    assert!(
        parsed
            .posts
            .contains(&"https://example.com/post/1".to_string())
    );
    assert!(
        !parsed
            .posts
            .iter()
            .any(|link| link.contains("other.example"))
    );
}

#[test]
fn parser_detection_uses_the_url_host() {
    assert_eq!(
        parser_for_url("https://e621.net/posts?tags=x").id,
        GalleryParserId::E621
    );
    assert_eq!(
        parser_for_url("https://safebooru.org/index.php").id,
        GalleryParserId::Safebooru
    );
    assert_eq!(
        parser_for_url("https://unknown.example/gallery").id,
        GalleryParserId::Generic
    );
}

#[test]
fn amp_encoded_links_are_decoded() {
    let links = extract_attribute_links(
        r#"<a href="index.php?page=post&amp;s=view&amp;id=5">x</a>"#,
        "https://rule34.xxx/index.php",
    );

    assert_eq!(
        links,
        vec!["https://rule34.xxx/index.php?page=post&s=view&id=5".to_string()]
    );
}
