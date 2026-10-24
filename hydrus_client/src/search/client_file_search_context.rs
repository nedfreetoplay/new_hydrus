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

fn parse_context<T: SerialisableObject + Default>(tuple: &Value, expected_type: u16) -> HydrusResult<T> {
    let values = tuple
        .as_array()
        .ok_or_else(|| serialisation("context must be a serialisable tuple"))?;

    let version = values.get(1).and_then(Value::as_u64).unwrap_or(1) as u16;
    let info = values.get(2).unwrap_or(&Value::Null);

    if values.first().and_then(Value::as_u64) != Some(expected_type as u64) {
        return Err(serialisation("unexpected serialisable type"));
    }

    let mut context = T::default();
    let (_, info) = context.update_serialisable_info(version, info.clone())?;
    context.initialise_from_serialisable_info(&info)?;

    Ok(context)
}

impl SerialisableObject for FileSearchContext {
    fn serialisable_type(&self) -> u16 {
        hydrus_serialisable::SERIALISABLE_TYPE_FILE_SEARCH_CONTEXT
    }

    fn serialisable_version(&self) -> u16 {
        SERIALISABLE_VERSION
    }

    fn serialisable_name(&self) -> &'static str {
        "File Search Context"
    }

    fn get_serialisable_info(&self) -> Value {
        let predicates: Vec<Value> = self
            .predicates
            .iter()
            .map(SerialisableObject::get_serialisable_tuple)
            .collect();

        json!([
            self.location_context.get_serialisable_tuple(),
            self.tag_context.get_serialisable_tuple(),
            self.search_type,
            predicates,
            self.complete,
        ])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let values = info
            .as_array()
            .ok_or_else(|| serialisation("FileSearchContext info must be a list"))?;

        self.location_context = parse_context(
            values.first().unwrap_or(&Value::Null),
            hydrus_serialisable::SERIALISABLE_TYPE_LOCATION_CONTEXT,
        )?;
        self.tag_context = parse_context(
            values.get(1).unwrap_or(&Value::Null),
            hydrus_serialisable::SERIALISABLE_TYPE_TAG_CONTEXT,
        )?;
        self.search_type = values.get(2).and_then(Value::as_i64).unwrap_or(0) as i32;
        self.complete = values.get(4).and_then(Value::as_bool).unwrap_or(false);

        self.predicates = Vec::new();

        for tuple in values.get(3).and_then(Value::as_array).cloned().unwrap_or_default() {
            let values = tuple.as_array().ok_or_else(|| serialisation("bad predicate tuple"))?;
            let mut predicate = Predicate::tag("");
            let version = values.get(1).and_then(Value::as_u64).unwrap_or(8) as u16;
            let info = values.get(2).cloned().unwrap_or(Value::Null);
            let (_, info) = predicate.update_serialisable_info(version, info)?;
            predicate.initialise_from_serialisable_info(&info)?;
            self.predicates.push(predicate);
        }

        Ok(())
    }

    fn update_serialisable_info(
        &self,
        version: u16,
        old_info: Value,
    ) -> HydrusResult<(u16, Value)> {
        if version >= SERIALISABLE_VERSION {
            return Ok((version, old_info));
        }

        Ok((SERIALISABLE_VERSION, old_info))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::client_number_test::{NUMBER_TEST_OPERATOR_EQUAL, NumberTest};

    #[test]
    fn empty_context_allows_everything() {
        let context = FileSearchContext::new();

        assert!(context.has_no_predicates());
        assert_eq!(context.get_summary(), "allows all files");
        assert!(!context.has_system_limit());
    }

    #[test]
    fn tags_and_namespaces_are_split_by_inclusivity() {
        let mut excluded = Predicate::tag("bad tag");
        excluded.set_inclusive(false);

        let mut context = FileSearchContext::new();
        context.set_predicates(vec![
            Predicate::tag("blue eyes"),
            excluded,
            Predicate::namespace("creator"),
        ]);

        assert_eq!(context.get_tags_to_include(), vec!["blue eyes"]);
        assert_eq!(context.get_tags_to_exclude(), vec!["bad tag"]);
        assert_eq!(context.get_namespaces_to_include(), vec!["creator"]);
    }

    #[test]
    fn serialisation_round_trips_nested_contexts() {
        let mut context = FileSearchContext::new();
        context.set_tag_service_key(b"local tags".to_vec());
        context.set_predicates(vec![Predicate::system_number(
            PREDICATE_TYPE_SYSTEM_SIZE,
            NumberTest::new(NUMBER_TEST_OPERATOR_EQUAL, 1024.0, None),
        )]);
        context.set_complete(true);

        let info = context.get_serialisable_info();
        let mut loaded = FileSearchContext::new();

        loaded.initialise_from_serialisable_info(&info).unwrap();

        assert_eq!(loaded, context);
        assert!(loaded.is_complete());
    }

    #[test]
    fn media_result_test_checks_inbox_and_mime() {
        use hydrus_core::files::hydrus_file_handling::FileInfo;
        use hydrus_core::hydrus_constants::Mime;

        let mut result = MediaResult::new(
            vec![1],
            FileInfo {
                size: 1024,
                mime: Mime::Png,
                width: None,
                height: None,
                duration_ms: None,
                num_frames: None,
                has_audio: false,
                num_words: None,
            },
        );
        result.locations_manager.inbox = true;

        let mut context = FileSearchContext::new();
        context.set_predicates(vec![
            Predicate::new(PREDICATE_TYPE_SYSTEM_INBOX, PredicateValue::None),
            Predicate::new(
                PREDICATE_TYPE_SYSTEM_MIME,
                PredicateValue::Numbers(vec![Mime::Png.as_int() as i32]),
            ),
        ]);

        assert!(context.test_media_result(&result));
    }

    #[test]
    fn summary_is_sorted_for_small_contexts() {
        let mut context = FileSearchContext::new();
        context.set_predicates(vec![Predicate::tag("zebra"), Predicate::tag("apple")]);

        assert_eq!(context.get_summary(), "apple, zebra");
    }
}
