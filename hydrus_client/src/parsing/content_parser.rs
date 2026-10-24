//! Content parsers from `hydrus/client/parsing/ClientParsing.py`.

use std::collections::BTreeSet;

use hydrus_core::hydrus_constants::ContentType;

use crate::parsing::formula::{ParseFormula, ParsingContext};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedValues {
    pub tags: BTreeSet<String>,
}

pub struct ContentParser<F: ParseFormula> {
    pub name: String,
    pub content_type: ContentType,
    pub formula: F,
    pub additional_info: Option<String>,
}

impl<F: ParseFormula> ContentParser<F> {
    pub fn new(
        name: impl Into<String>,
        content_type: ContentType,
        formula: F,
        additional_info: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            content_type,
            formula,
            additional_info,
        }
    }

    pub fn parse(
        &self,
        context: &ParsingContext,
        text: &str,
        collapse_newlines: bool,
    ) -> ParsedValues {
        if self.content_type != ContentType::Mappings {
            return ParsedValues::default();
        }

        let tags = self
            .formula
            .parse(context, text, collapse_newlines)
            .into_iter()
            .map(|tag| match &self.additional_info {
                None => tag,
                Some(namespace) => format!("{namespace}:{tag}"),
            })
            .collect();

        ParsedValues { tags }
    }

    pub fn to_string(&self) -> String {
        format!("{}: {}", self.name, self.content_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct DummyFormula {
        results: Vec<String>,
    }

    impl ParseFormula for DummyFormula {
        fn parse(
            &self,
            _context: &ParsingContext,
            _text: &str,
            _collapse_newlines: bool,
        ) -> Vec<String> {
            self.results.clone()
        }

        fn name(&self) -> &str {
            "dummy formula"
        }
    }

    fn formula() -> DummyFormula {
        DummyFormula {
            results: vec![
                "character:jane raider".to_string(),
                "double pistols".to_string(),
            ],
        }
    }

    fn tags(additional_info: Option<&str>) -> BTreeSet<String> {
        let parser = ContentParser::new(
            "test content parser",
            ContentType::Mappings,
            formula(),
            additional_info.map(str::to_string),
        );

        parser
            .parse(&Default::default(), "test parsing text", true)
            .tags
    }

    #[test]
    fn mappings_without_additional_info_keep_tags() {
        let tags = tags(None);

        assert!(tags.contains("character:jane raider"));
        assert!(tags.contains("double pistols"));
    }

    #[test]
    fn additional_info_prefixes_every_tag() {
        assert!(tags(Some("")).contains(":character:jane raider"));
        assert!(tags(Some("character")).contains("character:double pistols"));
        assert!(tags(Some("series")).contains("series:character:jane raider"));
    }

    #[test]
    fn non_mapping_content_is_not_returned_as_tags() {
        let parser = ContentParser::new("notes", ContentType::Notes, formula(), None);

        assert!(
            parser
                .parse(&Default::default(), "text", true)
                .tags
                .is_empty()
        );
    }
}
