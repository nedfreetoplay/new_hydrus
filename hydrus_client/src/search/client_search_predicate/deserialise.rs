//! Predicate deserialisation.

use hydrus_core::hydrus_exceptions::{HydrusResult, serialisation};
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::Value;

use crate::search::client_number_test::NumberTest;
use crate::search::predicate_constants::*;

use super::hex::hex_decode;
use super::{Predicate, PredicateValue};

pub(super) fn deserialise_value(
    predicate_type: PredicateType,
    value: &Value,
) -> HydrusResult<PredicateValue> {
    if value.is_null() {
        return Ok(PredicateValue::None);
    }

    if matches!(
        predicate_type,
        PredicateType::Tag
            | PredicateType::Namespace
            | PredicateType::Wildcard
            | PredicateType::Label
    ) {
        return Ok(PredicateValue::Text(
            value.as_str().unwrap_or_default().to_string(),
        ));
    }

    if predicate_type == PredicateType::SystemFileService {
        let values = value
            .as_array()
            .ok_or_else(|| serialisation("file service predicate value must be a list"))?;

        return Ok(PredicateValue::FileService {
            is_in: values.first().and_then(Value::as_bool).unwrap_or(true),
            status: values.get(1).and_then(Value::as_i64).unwrap_or(0) as i32,
            service_key: values
                .get(2)
                .and_then(Value::as_str)
                .and_then(hex_decode)
                .unwrap_or_default(),
        });
    }

    if matches!(
        predicate_type,
        PredicateType::SystemImportTime
            | PredicateType::SystemModifiedTime
            | PredicateType::SystemArchivedTime
            | PredicateType::SystemLastViewedTime
            | PredicateType::SystemTime
    ) {
        let values = value
            .as_array()
            .ok_or_else(|| serialisation("date predicate value must be a list"))?;

        return Ok(PredicateValue::Date {
            operator: values
                .first()
                .and_then(Value::as_str)
                .unwrap_or("=")
                .to_string(),
            date_type: values
                .get(1)
                .and_then(Value::as_str)
                .unwrap_or("delta")
                .to_string(),
            values: values
                .get(2)
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_i64)
                        .map(|value| value as i32)
                        .collect()
                })
                .unwrap_or_default(),
        });
    }

    if predicate_type == PredicateType::SystemRating {
        let values = value
            .as_array()
            .ok_or_else(|| serialisation("service predicate value must be a list"))?;
        return Ok(PredicateValue::Service {
            operator: values.first().and_then(Value::as_i64).unwrap_or(0) as i32,
            value: values.get(1).and_then(Value::as_f64),
            service_key: values
                .get(2)
                .and_then(Value::as_str)
                .and_then(hex_decode)
                .unwrap_or_default(),
        });
    }

    if predicate_type == PredicateType::SystemMime {
        return Ok(PredicateValue::Numbers(
            value
                .as_array()
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_i64)
                        .map(|v| v as i32)
                        .collect()
                })
                .unwrap_or_default(),
        ));
    }

    if predicate_type == PredicateType::SystemHash {
        return Ok(PredicateValue::Hashes(
            value
                .as_array()
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .filter_map(hex_decode)
                        .collect()
                })
                .unwrap_or_default(),
        ));
    }

    if predicate_type == PredicateType::OrContainer {
        let mut predicates = Vec::new();

        for tuple in value.as_array().cloned().unwrap_or_default() {
            if let Ok(predicate) =
                hydrus_serialisable::create_from_serialisable_tuple(&tuple, false)
            {
                if let Some(predicate) = predicate.as_any().downcast_ref::<Predicate>() {
                    predicates.push(predicate.clone());
                }
            }
        }

        return Ok(PredicateValue::Or(predicates));
    }

    if value.is_array() {
        let mut number = NumberTest::default();
        number.initialise_from_serialisable_info(value.get(2).unwrap_or(value))?;
        return Ok(PredicateValue::Number(number));
    }

    Ok(PredicateValue::None)
}
