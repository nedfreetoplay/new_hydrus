//! Parsing result descriptions from `hydrus/client/parsing/ClientParsingResults.py`.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContentKind {
    Url { url_type: i32, priority: i32 },
    Hash { hash_type: String, hash_encoding: String },
    Tag { namespace: Option<String> },
    Note { note_name: String },
    Timestamp { timestamp_type: Option<i32> },
    Title { priority: i32 },
    HttpHeaders { header_name: String },
    Variable { temp_variable_name: String },
    Veto,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParsableContentDescription {
    pub name: String,
    pub kind: ContentKind,
}

impl ParsableContentDescription {
    pub fn new(name: impl Into<String>, kind: ContentKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }

    pub fn get_shorthand_content_specific_info_string(&self) -> Option<String> {
        match &self.kind {
            ContentKind::Url { url_type, priority } => Some(format!("url type {url_type}, p{priority}")),
            ContentKind::Hash { hash_type, .. } => Some(hash_type.clone()),
            ContentKind::Tag { namespace } => namespace.clone(),
            ContentKind::Note { note_name } => Some(note_name.clone()),
            ContentKind::Timestamp { timestamp_type } => timestamp_type.map(|value| value.to_string()),
            ContentKind::Title { priority } => Some(format!("p{priority}")),
            ContentKind::HttpHeaders { header_name } => Some(header_name.clone()),
            ContentKind::Variable {
                temp_variable_name,
            } => Some(temp_variable_name.clone()),
            ContentKind::Veto => None,
        }
    }

    pub fn to_string(&self, parsed_text: Option<&str>) -> String {
        let mut text = match &self.kind {
            ContentKind::Url { .. } => "url".to_string(),
            ContentKind::Hash { .. } => "hash".to_string(),
            ContentKind::Tag { .. } => "tag".to_string(),
            ContentKind::Note { .. } => "note".to_string(),
            ContentKind::Timestamp { .. } => "timestamp".to_string(),
            ContentKind::Title { .. } => "title".to_string(),
            ContentKind::HttpHeaders { .. } => "http headers".to_string(),
            ContentKind::Variable { .. } => "variable".to_string(),
            ContentKind::Veto => "veto".to_string(),
        };

        text = format!("{}: {text}", self.name);

        if let Some(info) = self.get_shorthand_content_specific_info_string() {
            text.push_str(&format!(" ({info})"));
        }

        if let Some(parsed_text) = parsed_text {
            text.push_str(&format!(" -> {parsed_text}"));
        }

        text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedContent {
    pub description: ParsableContentDescription,
    pub parsed_text: String,
}

impl ParsedContent {
    pub fn new(description: ParsableContentDescription, parsed_text: impl Into<String>) -> Self {
        Self {
            description,
            parsed_text: parsed_text.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedPost {
    pub url: String,
    pub title: Option<String>,
    pub contents: Vec<ParsedContent>,
    pub tags: BTreeMap<String, Vec<String>>,
}

impl ParsedPost {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    pub fn add_content(&mut self, content: ParsedContent) {
        self.contents.push(content);
    }

    pub fn add_tag(&mut self, service_key: impl Into<String>, tag: impl Into<String>) {
        self.tags
            .entry(service_key.into())
            .or_default()
            .push(tag.into());
    }

    pub fn get_urls(&self) -> Vec<&str> {
        self.contents
            .iter()
            .filter(|content| matches!(content.description.kind, ContentKind::Url { .. }))
            .map(|content| content.parsed_text.as_str())
            .collect()
    }

    pub fn has_veto(&self) -> bool {
        self.contents
            .iter()
            .any(|content| content.description.kind == ContentKind::Veto)
    }
}

#[cfg(test)]
mod tests {
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
            description.get_shorthand_content_specific_info_string().as_deref(),
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
}
