//! Autocomplete parsing from `hydrus/client/search/ClientSearchAutocomplete.py`.

use crate::search::client_search_predicate::{Predicate, PredicateValue};
use crate::search::predicate_constants::{PREDICATE_TYPE_TAG, PREDICATE_TYPE_WILDCARD};
use crate::search::parse_system_predicate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagAutocompleteOptions {
    pub collapse_search_characters: bool,
    pub include_current: bool,
    pub include_pending: bool,
}

impl Default for TagAutocompleteOptions {
    fn default() -> Self {
        Self {
            collapse_search_characters: true,
            include_current: true,
            include_pending: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAutocompleteText {
    pub raw_input: String,
    pub inclusive: bool,
    pub tag_autocomplete_options: TagAutocompleteOptions,
}

impl ParsedAutocompleteText {
    pub fn new(raw_input: impl Into<String>, options: TagAutocompleteOptions) -> Self {
        Self {
            raw_input: raw_input.into(),
            inclusive: true,
            tag_autocomplete_options: options,
        }
    }

    pub fn get_search_text(
        &self,
        always_autocompleting: bool,
        allow_auto_wildcard_conversion: bool,
    ) -> String {
        let mut text = self.raw_input.trim().to_string();

        if text.starts_with('-') && !self.inclusive {
            text = text.trim_start_matches('-').trim_start().to_string();
        }

        if self.tag_autocomplete_options.collapse_search_characters {
            text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        }

        if always_autocompleting && allow_auto_wildcard_conversion && !text.contains('*') {
            text.push('*');
        }

        text
    }

    pub fn is_empty(&self) -> bool {
        self.get_search_text(false, false).is_empty()
    }

    pub fn is_namespace_search(&self) -> bool {
        let text = self.get_search_text(false, false);
        text.ends_with(':') && text.len() > 1
    }

    pub fn is_explicit_wildcard(&self, allow_auto_wildcard_conversion: bool) -> bool {
        let text = self.get_search_text(false, false);
        allow_auto_wildcard_conversion && text.contains('*')
    }

    pub fn is_tag_search(&self, allow_auto_wildcard_conversion: bool) -> bool {
        let text = self.get_search_text(false, false);

        !text.is_empty() && !text.starts_with("system:") && !self.is_namespace_search()
            && !self.is_explicit_wildcard(allow_auto_wildcard_conversion)
    }

    pub fn is_possible_system_predicate(&self) -> bool {
        self.get_search_text(false, false)
            .to_lowercase()
            .starts_with("system:")
    }

    pub fn is_valid_system_predicate(&self) -> bool {
        self.get_valid_system_predicates()
            .is_some_and(|predicates| !predicates.is_empty())
    }

    pub fn get_valid_system_predicates(&self) -> Option<Vec<Predicate>> {
        let text = self.get_search_text(false, false);

        if !text.to_lowercase().starts_with("system:") {
            return None;
        }

        parse_system_predicate(&text).ok().map(|predicate| vec![predicate])
    }

    pub fn get_immediate_file_search_predicate(
        &self,
        allow_auto_wildcard_conversion: bool,
    ) -> Option<Predicate> {
        if self.is_possible_system_predicate() {
            return self.get_valid_system_predicates().and_then(|mut predicates| {
                if predicates.len() == 1 {
                    predicates.pop()
                } else {
                    None
                }
            });
        }

        let text = self.get_search_text(false, false);

        if text.is_empty() {
            return None;
        }

        if self.is_explicit_wildcard(allow_auto_wildcard_conversion) {
            Some(Predicate::new(
                PREDICATE_TYPE_WILDCARD,
                PredicateValue::Text(text),
            ))
        } else if self.is_tag_search(allow_auto_wildcard_conversion) {
            Some(Predicate::new(PREDICATE_TYPE_TAG, PredicateValue::Text(text)))
        } else {
            None
        }
    }

    pub fn get_add_tag_predicate(&self) -> Option<Predicate> {
        let text = self.get_search_text(false, false);

        if text.is_empty() || text.starts_with("system:") {
            return None;
        }

        Some(Predicate::tag(text))
    }

    pub fn set_inclusive(&mut self, inclusive: bool) {
        self.inclusive = inclusive;
    }
}

/// Simplified `PredicateResultsCache`: callers feed it the predicates already
/// fetched for a page, and it filters them as the user types.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PredicateResultsCache {
    predicates: Vec<Predicate>,
}

impl PredicateResultsCache {
    pub fn new(predicates: impl IntoIterator<Item = Predicate>) -> Self {
        Self {
            predicates: predicates.into_iter().collect(),
        }
    }

    pub fn get_predicates(&self) -> &[Predicate] {
        &self.predicates
    }

    pub fn can_serve_tag_results(
        &self,
        parsed_autocomplete_text: &ParsedAutocompleteText,
        exact_match: bool,
    ) -> bool {
        if self.predicates.is_empty() {
            return false;
        }

        if parsed_autocomplete_text.is_empty() {
            return true;
        }

        let search_text = parsed_autocomplete_text.get_search_text(false, false);

        self.predicates.iter().any(|predicate| {
            let predicate_text = match &predicate.value {
                PredicateValue::Text(text) => text,
                _ => return false,
            };

            if exact_match {
                predicate_text == &search_text
            } else {
                predicate_text
                    .to_lowercase()
                    .contains(&search_text.to_lowercase())
            }
        })
    }

    pub fn filter_predicates(&self, search_text: &str) -> Vec<Predicate> {
        let search_text = search_text.to_lowercase();

        self.predicates
            .iter()
            .filter(|predicate| match &predicate.value {
                PredicateValue::Text(text) => text.to_lowercase().contains(&search_text),
                _ => false,
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> ParsedAutocompleteText {
        ParsedAutocompleteText::new(text, TagAutocompleteOptions::default())
    }

    #[test]
    fn whitespace_is_collapsed() {
        let text = parsed("  blue    eyes  ");

        assert_eq!(text.get_search_text(false, false), "blue eyes");
    }

    #[test]
    fn namespace_and_wildcard_are_recognised() {
        assert!(parsed("creator:").is_namespace_search());
        assert!(parsed("*eyes*").is_explicit_wildcard(true));
        assert!(!parsed("blue eyes").is_explicit_wildcard(true));
    }

    #[test]
    fn system_predicates_are_parsed() {
        let text = parsed("system:inbox");
        let predicates = text.get_valid_system_predicates().unwrap();

        assert_eq!(predicates.len(), 1);
        assert!(text.is_valid_system_predicate());
        assert!(text.is_possible_system_predicate());
    }

    #[test]
    fn immediate_predicate_converts_tags_and_wildcards() {
        let tag = parsed("blue eyes")
            .get_immediate_file_search_predicate(true)
            .unwrap();

        assert_eq!(tag.predicate_type, PREDICATE_TYPE_TAG);

        let wildcard = parsed("*eyes*")
            .get_immediate_file_search_predicate(true)
            .unwrap();

        assert_eq!(wildcard.predicate_type, PREDICATE_TYPE_WILDCARD);
    }

    #[test]
    fn add_tag_predicate_ignores_system_text() {
        assert!(parsed("system:inbox").get_add_tag_predicate().is_none());
        assert_eq!(
            parsed("blue eyes")
                .get_add_tag_predicate()
                .unwrap()
                .to_string(false),
            "blue eyes"
        );
    }

    #[test]
    fn cache_filters_by_substring() {
        let cache = PredicateResultsCache::new(vec![
            Predicate::tag("blue eyes"),
            Predicate::tag("blonde hair"),
            Predicate::tag("green eyes"),
        ]);

        assert_eq!(cache.filter_predicates("eyes").len(), 2);
        assert!(cache.can_serve_tag_results(&parsed("blue"), false));
        assert!(!cache.can_serve_tag_results(&parsed("red"), false));
    }
}
