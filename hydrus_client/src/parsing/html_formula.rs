//! Minimal HTML formula from `hydrus/client/parsing/ClientParsing.py`.
//!
//! Only the descending-tag rules and text/attribute fetching used by the
//! client's importers are implemented; this is intentionally a lighter parser
//! than Python's BeautifulSoup-based one.

use regex::Regex;

use crate::parsing::formula::{ParseFormula, ParsingContext, process_texts};
use crate::parsing::string_ops::{StringProcessor, decode_html_entities};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlRule {
    Descending(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlContent {
    Text,
    Attribute(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlFormula {
    pub name: String,
    pub rules: Vec<HtmlRule>,
    pub content: HtmlContent,
    pub processor: StringProcessor,
}

impl HtmlFormula {
    pub fn descending(tag_name: impl Into<String>, content: HtmlContent) -> Self {
        Self {
            name: String::new(),
            rules: vec![HtmlRule::Descending(tag_name.into())],
            content,
            processor: StringProcessor::default(),
        }
    }

    fn extract(&self, text: &str) -> Vec<String> {
        let mut current = text.to_string();

        for rule in &self.rules {
            let HtmlRule::Descending(tag_name) = rule;
            let tag = regex::escape(tag_name);

            let paired = Regex::new(&format!(r"(?is)<{tag}\b([^>]*)>(.*?)</{tag}\s*>"))
                .expect("escaped tag regex");
            let self_closing =
                Regex::new(&format!(r"(?is)<{tag}\b([^>]*)/>")).expect("escaped tag regex");

            if let Some(captures) = paired.captures(&current) {
                let attributes = captures
                    .get(1)
                    .map(|value| value.as_str().to_string())
                    .unwrap_or_default();
                let inner = captures
                    .get(2)
                    .map(|value| value.as_str().to_string())
                    .unwrap_or_default();

                current = format!("{attributes}\u{1f}{inner}");
            } else if let Some(captures) = self_closing.captures(&current) {
                let attributes = captures
                    .get(1)
                    .map(|value| value.as_str().to_string())
                    .unwrap_or_default();

                current = format!("{attributes}\u{1f}");
            } else {
                return Vec::new();
            }
        }

        let (attributes, inner) = current
            .split_once('\u{1f}')
            .unwrap_or(("", current.as_str()));

        match &self.content {
            HtmlContent::Text => {
                let stripped = Regex::new(r"(?is)<[^>]*>")
                    .expect("static regex")
                    .replace_all(inner, "");
                vec![decode_html_entities(stripped.trim())]
            }
            HtmlContent::Attribute(name) => find_attribute(attributes, name)
                .map(|value| vec![decode_html_entities(&value)])
                .unwrap_or_default(),
        }
    }
}

fn find_attribute(attributes: &str, name: &str) -> Option<String> {
    let pattern = format!(
        r#"(?is)\b{}\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#,
        regex::escape(name)
    );
    let captures = Regex::new(&pattern).ok()?.captures(attributes)?;

    (1..=3)
        .filter_map(|index| captures.get(index))
        .next()
        .map(|value| value.as_str().to_string())
}

impl ParseFormula for HtmlFormula {
    fn parse(&self, _context: &ParsingContext, text: &str, collapse_newlines: bool) -> Vec<String> {
        process_texts(self.extract(text), &self.processor, collapse_newlines)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::formula::JsonFormula;
    use crate::parsing::formula::{JsonContent, JsonRule, NestedFormula};
    use crate::parsing::string_match::StringMatch;

    fn json_rules() -> Vec<JsonRule> {
        vec![
            JsonRule::DictKey(StringMatch::fixed("test")),
            JsonRule::IndexedItem(1),
        ]
    }

    #[test]
    fn descending_attribute_is_decoded() {
        let formula =
            HtmlFormula::descending("muh_tag", HtmlContent::Attribute("quantity".to_string()));

        let html = r#"<div><muh_tag quantity="&gt;implying">hello</muh_tag></div>"#;

        assert_eq!(
            formula.parse(&Default::default(), html, true),
            [">implying"]
        );
    }

    #[test]
    fn nested_json_then_html_decodes_entities() {
        let payload = serde_json::json!({
            "test": ["hello", r#"<div><muh_tag quantity="&gt;implying">hello</muh_tag></div>"#],
        });

        let main = JsonFormula::new(json_rules(), JsonContent::String);
        let sub =
            HtmlFormula::descending("muh_tag", HtmlContent::Attribute("quantity".to_string()));
        let formula = NestedFormula::new(Box::new(main), Box::new(sub));

        assert_eq!(
            formula.parse(&Default::default(), &payload.to_string(), true),
            [">implying"]
        );
    }
}
