//! Predicate serialisation.

use std::any::Any;

use hydrus_core::hydrus_exceptions::{HydrusResult, serialisation};
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::{Value, json};

use crate::search::predicate_constants::*;

use super::deserialise::deserialise_value;
use super::hex::hex_encode;
use super::{Predicate, PredicateValue, SERIALISABLE_VERSION};

impl SerialisableObject for Predicate {
    fn serialisable_type(&self) -> u16 {
        hydrus_serialisable::SERIALISABLE_TYPE_PREDICATE
    }

    fn serialisable_version(&self) -> u16 {
        SERIALISABLE_VERSION
    }

    fn serialisable_name(&self) -> &'static str {
        "File Search Predicate"
    }

    fn get_serialisable_info(&self) -> Value {
        json!([
            self.predicate_type.as_int(),
            serialise_value(self.predicate_type, &self.value),
            self.inclusive
        ])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let values = info
            .as_array()
            .ok_or_else(|| serialisation("Predicate info must be a list"))?;

        self.predicate_type =
            PredicateType::try_from_int(values.first().and_then(Value::as_i64).unwrap_or(0) as i32)
                .unwrap_or(PredicateType::Tag);
        self.inclusive = values.get(2).and_then(Value::as_bool).unwrap_or(true);
        self.value = deserialise_value(self.predicate_type, values.get(1).unwrap_or(&Value::Null))?;

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

fn serialise_value(_predicate_type: PredicateType, value: &PredicateValue) -> Value {
    match value {
        PredicateValue::None => Value::Null,
        PredicateValue::Text(text) => json!(text),
        PredicateValue::Number(number) => number.to_json_value(),
        PredicateValue::Numbers(numbers) => json!(numbers),
        PredicateValue::Hashes(hashes) => {
            json!(
                hashes
                    .iter()
                    .map(|hash| hex_encode(hash))
                    .collect::<Vec<_>>()
            )
        }
        PredicateValue::Service {
            operator,
            value,
            service_key,
        } => json!([operator, value, hex_encode(service_key)]),
        PredicateValue::FileService {
            is_in,
            status,
            service_key,
        } => json!([is_in, status, hex_encode(service_key)]),
        PredicateValue::Date {
            operator,
            date_type,
            values,
        } => json!([operator, date_type, values]),
        PredicateValue::Or(predicates) => Value::Array(
            predicates
                .iter()
                .map(SerialisableObject::get_serialisable_tuple)
                .collect(),
        ),
    }
}
