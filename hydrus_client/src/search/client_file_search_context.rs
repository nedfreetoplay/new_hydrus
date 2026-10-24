//! `FileSearchContext` from `hydrus/client/search/ClientSearchFileSearchContext.py`.

use std::any::Any;

use hydrus_core::hydrus_exceptions::{HydrusResult, serialisation};
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::{Value, json};

use crate::client_location::LocationContext;
use crate::media::MediaResult;
use crate::search::client_file_system_predicates::FileSystemPredicates;
use crate::search::client_search_predicate::{Predicate, PredicateValue};
use crate::search::client_search_tag_context::TagContext;
use crate::search::predicate_constants::*;

pub const SEARCH_TYPE_AND: i32 = 0;
pub const SEARCH_TYPE_OR: i32 = 1;
pub const SERIALISABLE_VERSION: u16 = 5;

#[derive(Debug, Clone, PartialEq)]
pub struct FileSearchContext {
    location_context: LocationContext,
    tag_context: TagContext,
    search_type: i32,
    predicates: Vec<Predicate>,
    complete: bool,
}

impl Default for FileSearchContext {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSearchContext {
    pub fn new() -> Self {
        Self {
            location_context: LocationContext::default(),
            tag_context: TagContext::default(),
            search_type: SEARCH_TYPE_AND,
            predicates: Vec::new(),
            complete: false,
        }
    }

    pub fn get_location_context(&self) -> &LocationContext {
        &self.location_context
    }

    pub fn get_tag_context(&self) -> &TagContext {
        &self.tag_context
    }

    pub fn get_predicates(&self) -> &[Predicate] {
        &self.predicates
    }

    pub fn set_predicates(&mut self, predicates: Vec<Predicate>) {
        self.predicates = predicates;
    }

    pub fn set_location_context(&mut self, location_context: LocationContext) {
        self.location_context = location_context;
    }

    pub fn set_tag_context(&mut self, tag_context: TagContext) {
        self.tag_context = tag_context;
    }

    pub fn set_tag_service_key(&mut self, service_key: Vec<u8>) {
        self.tag_context.service_key = service_key.clone();
        self.tag_context.display_service_key = service_key;
    }

    pub fn set_search_type(&mut self, search_type: i32) {
        self.search_type = search_type;
    }

    pub fn get_search_type(&self) -> i32 {
        self.search_type
    }

    pub fn set_complete(&mut self, complete: bool) {
        self.complete = complete;
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn has_no_predicates(&self) -> bool {
        self.predicates.is_empty()
    }

    pub fn is_just_system_everything(&self) -> bool {
        self.predicates.len() == 1
            && self.predicates[0].predicate_type == PREDICATE_TYPE_SYSTEM_EVERYTHING
    }

    pub fn get_system_predicates(&self) -> FileSystemPredicates {
        FileSystemPredicates::new(&self.predicates)
    }

    pub fn get_tags_to_include(&self) -> Vec<String> {
        self.texts(PREDICATE_TYPE_TAG, true)
    }

    pub fn get_tags_to_exclude(&self) -> Vec<String> {
        self.texts(PREDICATE_TYPE_TAG, false)
    }

    pub fn get_namespaces_to_include(&self) -> Vec<String> {
        self.texts(PREDICATE_TYPE_NAMESPACE, true)
    }

    pub fn get_namespaces_to_exclude(&self) -> Vec<String> {
        self.texts(PREDICATE_TYPE_NAMESPACE, false)
    }

    pub fn get_wildcards_to_include(&self) -> Vec<String> {
        self.texts(PREDICATE_TYPE_WILDCARD, true)
    }

    pub fn get_wildcards_to_exclude(&self) -> Vec<String> {
        self.texts(PREDICATE_TYPE_WILDCARD, false)
    }

    pub fn get_or_predicates(&self) -> Vec<&Predicate> {
        self.predicates
            .iter()
            .filter(|predicate| predicate.predicate_type == PREDICATE_TYPE_OR_CONTAINER)
            .collect()
    }

    fn texts(&self, predicate_type: i32, inclusive: bool) -> Vec<String> {
        self.predicates
            .iter()
            .filter(|predicate| {
                predicate.predicate_type == predicate_type && predicate.inclusive == inclusive
            })
            .filter_map(|predicate| match &predicate.value {
                PredicateValue::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn get_summary(&self) -> String {
        if self.predicates.is_empty() {
            return "allows all files".to_string();
        }

        if self.predicates.len() > 3 {
            return format!("{} predicates", self.predicates.len());
        }

        let mut strings: Vec<String> = self
            .predicates
            .iter()
            .map(|predicate| predicate.to_string(true))
            .collect();
        strings.sort();

        strings.join(", ")
    }

    /// Minimal predicate test for the fields already modelled on `MediaResult`.
    pub fn test_media_result(&self, media_result: &MediaResult) -> bool {
        let system = self.get_system_predicates();

        if system.must_be_inbox() && !media_result.get_inbox() {
            return false;
        }

        if system.must_be_archive() && media_result.get_inbox() {
            return false;
        }

        if system.has_allowed_filetypes() && !system.mimes.contains(&media_result.get_mime()) {
            return false;
        }

        for predicate in &self.predicates {
            if predicate.predicate_type == PREDICATE_TYPE_SYSTEM_SIZE {
                if let PredicateValue::Number(number) = &predicate.value {
                    if !number.test(Some(media_result.get_size() as f64), None) {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Kept for API parity; limits are applied by the DB layer.
    pub fn get_limit(&self, apply_implicit_limit: bool) -> Option<i64> {
        self.get_system_predicates().get_limit(apply_implicit_limit)
    }

    pub fn has_system_limit(&self) -> bool {
        self.predicates
            .iter()
            .any(|predicate| predicate.predicate_type == PREDICATE_TYPE_SYSTEM_LIMIT)
    }
}

mod serialisation;
pub use serialisation::*;

#[cfg(test)]
mod tests;
