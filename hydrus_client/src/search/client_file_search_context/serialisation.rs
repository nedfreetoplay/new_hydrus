//! Split out from the parent module.

use crate::client_location::LocationContext;
use crate::media::MediaResult;
use crate::search::client_file_system_predicates::FileSystemPredicates;
use crate::search::client_search_predicate::{Predicate, PredicateValue};
use crate::search::client_search_tag_context::TagContext;
use crate::search::predicate_constants::*;
use hydrus_core::hydrus_exceptions::{HydrusResult, serialisation};
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::{Value, json};
use std::any::Any;

use super::*;

fn parse_context<T: SerialisableObject + Default>(
    tuple: &Value,
    expected_type: u16,
) -> HydrusResult<T> {
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

        for tuple in values
            .get(3)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            let values = tuple
                .as_array()
                .ok_or_else(|| serialisation("bad predicate tuple"))?;
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
