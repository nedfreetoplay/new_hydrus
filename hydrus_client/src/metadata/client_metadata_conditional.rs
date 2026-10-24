//! Metadata conditions from `hydrus/client/metadata/ClientMetadataConditional.py`.

use std::any::Any;

use hydrus_core::hydrus_exceptions::{HydrusResult, serialisation};
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::Value;

use crate::media::MediaResult;
use crate::search::FileSearchContext;

pub const SERIALISABLE_VERSION: u16 = 1;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MetadataConditional {
    file_search_context: FileSearchContext,
}

impl MetadataConditional {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_file_search_context(&self) -> &FileSearchContext {
        &self.file_search_context
    }

    pub fn set_file_search_context(&mut self, file_search_context: FileSearchContext) {
        self.file_search_context = file_search_context;
    }

    pub fn get_summary(&self) -> String {
        self.file_search_context.get_summary()
    }

    pub fn test(&self, media_result: &MediaResult) -> bool {
        self.file_search_context.test_media_result(media_result)
    }
}

impl SerialisableObject for MetadataConditional {
    fn serialisable_type(&self) -> u16 {
        hydrus_serialisable::SERIALISABLE_TYPE_METADATA_CONDITIONAL
    }

    fn serialisable_version(&self) -> u16 {
        SERIALISABLE_VERSION
    }

    fn serialisable_name(&self) -> &'static str {
        "Metadata Conditional"
    }

    fn get_serialisable_info(&self) -> Value {
        self.file_search_context.get_serialisable_tuple()
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let values = info
            .as_array()
            .ok_or_else(|| serialisation("MetadataConditional info must be a tuple"))?;

        let context_info = match values.first().and_then(Value::as_u64) {
            Some(type_id) if type_id == hydrus_serialisable::SERIALISABLE_TYPE_FILE_SEARCH_CONTEXT as u64 => {
                values.get(2).cloned().unwrap_or(Value::Null)
            }
            _ => info.clone(),
        };

        self.file_search_context = FileSearchContext::new();
        self.file_search_context
            .initialise_from_serialisable_info(&context_info)?;

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub fn register_serialisable_type() -> bool {
    hydrus_core::hydrus_serialisable::register_serialisable_type(
        hydrus_serialisable::SERIALISABLE_TYPE_METADATA_CONDITIONAL,
        || Box::new(MetadataConditional::new()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::predicate_constants::PREDICATE_TYPE_SYSTEM_INBOX;
    use crate::search::{Predicate, PredicateValue};
    use hydrus_core::files::hydrus_file_handling::FileInfo;
    use hydrus_core::hydrus_constants::Mime;

    fn result(inbox: bool) -> MediaResult {
        let mut result = MediaResult::new(
            vec![1],
            FileInfo {
                size: 1,
                mime: Mime::Png,
                width: None,
                height: None,
                duration_ms: None,
                num_frames: None,
                has_audio: false,
                num_words: None,
            },
        );
        result.locations_manager.inbox = inbox;
        result
    }

    #[test]
    fn empty_condition_matches_everything() {
        let conditional = MetadataConditional::new();

        assert!(conditional.test(&result(true)));
        assert_eq!(conditional.get_summary(), "allows all files");
    }

    #[test]
    fn inbox_condition_filters_media() {
        let mut context = FileSearchContext::new();
        context.set_predicates(vec![Predicate::new(
            PREDICATE_TYPE_SYSTEM_INBOX,
            PredicateValue::None,
        )]);

        let mut conditional = MetadataConditional::new();
        conditional.set_file_search_context(context);

        assert!(conditional.test(&result(true)));
        assert!(!conditional.test(&result(false)));
        assert_eq!(conditional.get_summary(), "inbox");
    }

    #[test]
    fn serialisation_round_trips() {
        let mut context = FileSearchContext::new();
        context.set_predicates(vec![Predicate::tag("blue eyes")]);

        let mut conditional = MetadataConditional::new();
        conditional.set_file_search_context(context);

        let info = conditional.get_serialisable_info();
        let mut loaded = MetadataConditional::new();

        loaded.initialise_from_serialisable_info(&info).unwrap();

        assert_eq!(loaded, conditional);
    }
}
