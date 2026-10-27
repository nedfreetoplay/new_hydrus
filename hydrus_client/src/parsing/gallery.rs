//! Site-aware gallery page parsing.
//!
//! Hydrus uses a database of URL classes and separate gallery/page parsers.
//! The GUI keeps a small built-in registry for the most common gallery sites so
//! a user can paste a gallery URL and get files without configuring a parser.
//! The generic parser keeps the old "follow links and collect media" fallback
//! for everything else. All functions are pure and take already-fetched HTML,
//! which keeps them unit-testable and lets the network worker own all I/O.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use regex::Regex;
use reqwest::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalleryParserId {
    Generic,
    Danbooru,
    Gelbooru,
    E621,
    Rule34,
    Safebooru,
    Pixiv,
}

impl GalleryParserId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Danbooru => "danbooru",
            Self::Gelbooru => "gelbooru",
            Self::E621 => "e621",
            Self::Rule34 => "rule34",
            Self::Safebooru => "safebooru",
            Self::Pixiv => "pixiv",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "auto" | "generic" => Some(Self::Generic),
            "danbooru" => Some(Self::Danbooru),
            "gelbooru" => Some(Self::Gelbooru),
            "e621" | "e-621" => Some(Self::E621),
            "rule34" | "rule 34" => Some(Self::Rule34),
            "safebooru" => Some(Self::Safebooru),
            "pixiv" => Some(Self::Pixiv),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GalleryParser {
    pub id: GalleryParserId,
    pub title: &'static str,
    pub hosts: &'static [&'static str],
    /// Gallery URL template; `{query}` is replaced with the encoded search.
    pub search_template: &'static str,
    /// Regex matching the `href`/`src` value of a post link.
    pub post_link: &'static str,
    /// Regex matching page navigation links.
    pub next_link: &'static str,
}

pub const GALLERY_PARSERS: &[GalleryParser] = &[
    GalleryParser {
        id: GalleryParserId::Generic,
        title: "generic (follow page links)",
        hosts: &[],
        search_template: "https://example.com/search?q={query}",
        post_link: "",
        next_link: "",
    },
    GalleryParser {
        id: GalleryParserId::Danbooru,
        title: "danbooru",
        hosts: &["danbooru.donmai.us", "safebooru.donmai.us"],
        search_template: "https://danbooru.donmai.us/posts?tags={query}",
        post_link: r#"(?i)/posts/\d+"#,
        next_link: r#"(?i)(?:/posts\?[^"']*\bpage=\d+|rel=["']next["'])"#,
    },
    GalleryParser {
        id: GalleryParserId::Gelbooru,
        title: "gelbooru",
        hosts: &["gelbooru.com", "www.gelbooru.com"],
        search_template: "https://gelbooru.com/index.php?page=post&s=list&tags={query}",
        post_link: r#"(?i)index\.php\?page=post(?:&amp;|&)s=view(?:&amp;|&)id=\d+"#,
        next_link: r#"(?i)(?:page=post[^"']*\bpid=\d+|rel=["']next["'])"#,
    },
    GalleryParser {
        id: GalleryParserId::E621,
        title: "e621",
        hosts: &["e621.net", "www.e621.net"],
        search_template: "https://e621.net/posts?tags={query}",
        post_link: r#"(?i)/posts/\d+"#,
        next_link: r#"(?i)(?:/posts\?[^"']*\bpage=\d+|rel=["']next["'])"#,
    },
    GalleryParser {
        id: GalleryParserId::Rule34,
        title: "rule34",
        hosts: &["rule34.xxx", "www.rule34.xxx"],
        search_template: "https://rule34.xxx/index.php?page=post&s=list&tags={query}",
        post_link: r#"(?i)index\.php\?page=post(?:&amp;|&)s=view(?:&amp;|&)id=\d+"#,
        next_link: r#"(?i)(?:page=post[^"']*\bpid=\d+|rel=["']next["'])"#,
    },
    GalleryParser {
        id: GalleryParserId::Safebooru,
        title: "safebooru",
        hosts: &["safebooru.org", "www.safebooru.org"],
        search_template: "https://safebooru.org/index.php?page=post&s=list&tags={query}",
        post_link: r#"(?i)index\.php\?page=post(?:&amp;|&)s=view(?:&amp;|&)id=\d+"#,
        next_link: r#"(?i)(?:page=post[^"']*\bpid=\d+|rel=["']next["'])"#,
    },
    GalleryParser {
        id: GalleryParserId::Pixiv,
        title: "pixiv",
        hosts: &["pixiv.net", "www.pixiv.net"],
        search_template: "https://www.pixiv.net/en/tags/{query}",
        post_link: r#"(?i)/artworks/\d+"#,
        next_link: r#"(?i)(?:/artworks/\d+/[^"']*|rel=["']next["'])"#,
    },
];

pub fn parser_for_url(url: &str) -> &'static GalleryParser {
    let host = Url::parse(url).ok().and_then(|url| {
        url.host_str()
            .map(|host| host.trim_start_matches("www.").to_string())
    });

    if let Some(host) = host {
        if let Some(parser) = GALLERY_PARSERS.iter().find(|parser| {
            parser
                .hosts
                .iter()
                .any(|candidate| candidate.trim_start_matches("www.") == host)
        }) {
            return parser;
        }
    }

    &GALLERY_PARSERS[0]
}

pub fn parser_by_id(id: &str) -> &'static GalleryParser {
    GalleryParserId::from_str(id)
        .and_then(|id| GALLERY_PARSERS.iter().find(|parser| parser.id == id))
        .unwrap_or(&GALLERY_PARSERS[0])
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ParsedGalleryPage {
    pub files: Vec<String>,
    pub posts: Vec<String>,
    pub next_pages: Vec<String>,
    pub is_post: bool,
}

/// Parses one gallery listing page. The caller fetches every `posts` entry and
/// calls [`parse_post_page`], then follows `next_pages` up to its own limit.
pub fn parse_gallery_page(html: &str, base: &str, parser: &GalleryParser) -> ParsedGalleryPage {
    let mut files: BTreeSet<String> = extract_media_links(html, base).into_iter().collect();
    let mut posts = BTreeSet::new();
    let mut next_pages = BTreeSet::new();

    if !parser.post_link.is_empty() {
        for link in extract_attribute_links(html, base) {
            if matches_pattern(parser.post_link, &link) {
                posts.insert(link);
            }
        }
    }

    if !parser.next_link.is_empty() {
        for link in extract_attribute_links(html, base) {
            if matches_pattern(parser.next_link, &link) && !posts.contains(&link) {
                next_pages.insert(link);
            }
        }
    }

    let is_post = parser.id != GalleryParserId::Generic
        && posts.iter().any(|post| same_url(post, base))
        || (parser.id != GalleryParserId::Generic && posts.is_empty() && !files.is_empty());

    if is_post {
        files.extend(parse_post_page(html, base, parser));
        posts.clear();
        next_pages.clear();
    } else if parser.id == GalleryParserId::Generic {
        // The generic parser treats every same-host non-media link as a page to
        // inspect. The worker limits how many it follows.
        for link in extract_attribute_links(html, base) {
            if !is_media_url(&link)
                && same_host(&link, base)
                && is_probable_page_link(&link)
                && !next_pages.contains(&link)
            {
                posts.insert(link);
            }
        }
    }

    ParsedGalleryPage {
        files: files.into_iter().collect(),
        posts: posts.into_iter().collect(),
        next_pages: next_pages.into_iter().collect(),
        is_post,
    }
}

/// Extracts the media URL (or URLs, for multi-page posts) from a post page.
pub fn parse_post_page(html: &str, base: &str, parser: &GalleryParser) -> Vec<String> {
    let mut files: BTreeSet<String> = BTreeSet::new();

    // Explicit metadata is much more reliable than guessing from thumbnails.
    for pattern in meta_media_patterns() {
        for capture in pattern.captures_iter(html) {
            if let Some(value) = capture.get(1) {
                if let Some(url) = absolute_link(base, value.as_str()) {
                    if is_media_url(&url) {
                        files.insert(url);
                    }
                }
            }
        }
    }

    // JSON payloads used by several sites (pixiv, danbooru variants).
    for pattern in json_media_patterns() {
        for capture in pattern.captures_iter(html) {
            if let Some(value) = capture.get(1) {
                if let Some(url) = absolute_link(base, value.as_str()) {
                    if is_media_url(&url) {
                        files.insert(url);
                    }
                }
            }
        }
    }

    // Site-specific embedded elements, e.g. gelbooru's `id="image"`.
    for link in extract_attribute_links(html, base) {
        if is_media_url(&link) {
            files.insert(link);
        }
    }

    // The generic parser should not turn a post's tags into media. The media
    // filter above already handles that.
    let _ = parser;

    files.into_iter().collect()
}

/// Every link found in `href`, `src`, `data-src` and `data-original` attributes.
pub fn extract_attribute_links(html: &str, base: &str) -> Vec<String> {
    static LINK: OnceLock<Regex> = OnceLock::new();

    let regex = LINK.get_or_init(|| {
        Regex::new(r#"(?is)(?:href|src|data-src|data-original)\s*=\s*["']([^"']+)["']"#)
            .expect("valid link regex")
    });

    let mut links = Vec::new();
    let mut seen = BTreeSet::new();

    for capture in regex.captures_iter(html) {
        let Some(raw) = capture.get(1) else {
            continue;
        };

        if let Some(link) = absolute_link(base, raw.as_str()) {
            if seen.insert(link.clone()) {
                links.push(link);
            }
        }
    }

    links
}

/// Media links present in a page, including OpenGraph images and JSON fields.
pub fn extract_media_links(html: &str, base: &str) -> Vec<String> {
    let mut files: BTreeSet<String> = BTreeSet::new();

    for pattern in meta_media_patterns() {
        for capture in pattern.captures_iter(html) {
            if let Some(value) = capture.get(1) {
                if let Some(url) = absolute_link(base, value.as_str()) {
                    if is_media_url(&url) {
                        files.insert(url);
                    }
                }
            }
        }
    }

    for link in extract_attribute_links(html, base) {
        if is_media_url(&link) {
            files.insert(link);
        }
    }

    files.into_iter().collect()
}

pub fn is_media_url(url: &str) -> bool {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .to_ascii_lowercase();

    let extension = path.rsplit('.').next().unwrap_or_default();

    matches!(
        extension,
        "jpg"
            | "jpeg"
            | "png"
            | "gif"
            | "webp"
            | "bmp"
            | "tif"
            | "tiff"
            | "ico"
            | "qoi"
            | "avif"
            | "heic"
            | "jxl"
            | "svg"
            | "mp4"
            | "webm"
            | "mkv"
            | "mov"
            | "m4v"
            | "avi"
            | "mp3"
            | "flac"
            | "wav"
            | "ogg"
            | "opus"
            | "m4a"
            | "pdf"
            | "zip"
            | "rar"
            | "7z"
    )
}

fn meta_media_patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

    PATTERNS.get_or_init(|| {
        [
            r#"(?is)<meta[^>]+(?:property|name)\s*=\s*["'](?:og:image|og:video|twitter:image|twitter:player:stream)["'][^>]+content\s*=\s*["']([^"']+)["']"#,
            r#"(?is)<meta[^>]+content\s*=\s*["']([^"']+)["'][^>]+(?:property|name)\s*=\s*["'](?:og:image|og:video|twitter:image)["']"#,
            r#"(?is)<(?:video|source)[^>]+src\s*=\s*["']([^"']+)["']"#,
            r#"(?is)data-file-url\s*=\s*["']([^"']+)["']"#,
        ]
        .into_iter()
        .filter_map(|pattern| Regex::new(pattern).ok())
        .collect()
    })
}

fn json_media_patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

    PATTERNS.get_or_init(|| {
        [
            r#"(?is)"(?:original|file_url|fileUrl|source|url|medium)"\s*:\s*"([^"]+\.(?:jpg|jpeg|png|gif|webp|mp4|webm|zip))(?:\?[^"]*)?""#,
        ]
        .into_iter()
        .filter_map(|pattern| Regex::new(pattern).ok())
        .collect()
    })
}

fn matches_pattern(pattern: &str, value: &str) -> bool {
    static CACHE: OnceLock<std::sync::Mutex<std::collections::BTreeMap<String, Regex>>> =
        OnceLock::new();

    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(Default::default()));
    let mut cache = cache.lock().unwrap();

    let regex = cache
        .entry(pattern.to_string())
        .or_insert_with(|| Regex::new(pattern).expect("gallery parser regex is valid"));

    regex.is_match(value)
}

fn absolute_link(base: &str, raw: &str) -> Option<String> {
    let raw = raw.trim();

    if raw.is_empty()
        || raw.starts_with('#')
        || raw.starts_with("javascript:")
        || raw.starts_with("mailto:")
        || raw.starts_with("data:")
        || raw.starts_with("blob:")
    {
        return None;
    }

    let raw = raw
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    let base = Url::parse(base).ok()?;
    let mut url = base.join(&raw).ok()?;

    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }

    url.set_fragment(None);

    Some(url.to_string())
}

fn is_probable_page_link(link: &str) -> bool {
    let path = link
        .split(['?', '#'])
        .next()
        .unwrap_or(link)
        .to_ascii_lowercase();

    let extension = path.rsplit('.').next().unwrap_or_default();

    !matches!(
        extension,
        "css"
            | "js"
            | "json"
            | "xml"
            | "rss"
            | "ico"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "eot"
            | "map"
    )
}

fn same_url(a: &str, b: &str) -> bool {
    let normalize = |value: &str| {
        value
            .split('#')
            .next()
            .unwrap_or(value)
            .trim_end_matches('/')
            .to_ascii_lowercase()
    };

    normalize(a) == normalize(b)
}

fn same_host(a: &str, b: &str) -> bool {
    let host = |value: &str| {
        Url::parse(value)
            .ok()
            .and_then(|url| url.host_str().map(|host| host.to_ascii_lowercase()))
    };

    match (host(a), host(b)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests;
