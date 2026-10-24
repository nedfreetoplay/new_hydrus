//! Search predicates from `hydrus/client/search/ClientSearchPredicate.py`.

use std::any::Any;

use hydrus_core::hydrus_constants::Mime;
use hydrus_core::hydrus_exceptions::{HydrusResult, serialisation};
use hydrus_core::hydrus_serialisable::{self, SerialisableObject};
use serde_json::{Value, json};

use crate::search::client_number_test::NumberTest;
use crate::search::predicate_constants::*;

pub const SERIALISABLE_VERSION: u16 = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PredicateCount {
    pub min_current_count: i64,
    pub min_pending_count: i64,
    pub max_current_count: Option<i64>,
    pub max_pending_count: Option<i64>,
}

impl PredicateCount {
    pub fn get_min_count(&self) -> i64 {
        self.min_current_count + self.min_pending_count
    }

    pub fn get_suffix_string(&self) -> String {
        if self.get_min_count() > 0 {
            format!("({})", self.get_min_count())
        } else {
            String::new()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PredicateValue {
    None,
    Text(String),
    Number(NumberTest),
    Numbers(Vec<i32>),
    Hashes(Vec<Vec<u8>>),
    Service {
        operator: i32,
        value: Option<f64>,
        service_key: Vec<u8>,
    },
    FileService {
        is_in: bool,
        status: i32,
        service_key: Vec<u8>,
    },
    Date {
        operator: String,
        date_type: String,
        values: Vec<i32>,
    },
    Or(Vec<Predicate>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Predicate {
    pub predicate_type: i32,
    pub value: PredicateValue,
    pub inclusive: bool,
    pub count: PredicateCount,
}

impl Predicate {
    pub fn new(predicate_type: i32, value: PredicateValue) -> Self {
        Self {
            predicate_type,
            value,
            inclusive: true,
            count: PredicateCount::default(),
        }
    }

    pub fn tag(tag: impl Into<String>) -> Self {
        Self::new(PREDICATE_TYPE_TAG, PredicateValue::Text(tag.into()))
    }

    pub fn namespace(namespace: impl Into<String>) -> Self {
        Self::new(
            PREDICATE_TYPE_NAMESPACE,
            PredicateValue::Text(namespace.into()),
        )
    }

    pub fn system_number(predicate_type: i32, number_test: NumberTest) -> Self {
        Self::new(predicate_type, PredicateValue::Number(number_test))
    }

    pub fn get_type(&self) -> i32 {
        self.predicate_type
    }

    pub fn get_value(&self) -> &PredicateValue {
        &self.value
    }

    pub fn get_inclusive(&self) -> bool {
        self.inclusive
    }

    pub fn set_inclusive(&mut self, inclusive: bool) {
        if NON_INCLUSIVE_CAPABLE_PREDICATE_TYPES.contains(&self.predicate_type) {
            self.inclusive = inclusive;
        }
    }

    pub fn get_count(&self) -> PredicateCount {
        self.count
    }

    pub fn get_namespace(&self) -> Option<&str> {
        match &self.value {
            PredicateValue::Text(text) => text.split_once(':').map(|(namespace, _)| namespace),
            _ => None,
        }
    }

    pub fn get_texts_and_namespaces(&self) -> (Option<String>, Option<String>) {
        match (&self.value, self.predicate_type) {
            (PredicateValue::Text(text), PREDICATE_TYPE_TAG | PREDICATE_TYPE_PARENT) => {
                match text.split_once(':') {
                    Some((namespace, subtag)) => {
                        (Some(subtag.to_string()), Some(namespace.to_string()))
                    }
                    None => (Some(text.clone()), None),
                }
            }
            (PredicateValue::Text(text), PREDICATE_TYPE_NAMESPACE) => (None, Some(text.clone())),
            _ => (None, None),
        }
    }

    pub fn get_matchable_search_texts(&self) -> Vec<String> {
        match &self.value {
            PredicateValue::Text(text) => vec![text.clone()],
            _ => Vec::new(),
        }
    }

    pub fn get_copy(&self) -> Self {
        self.clone()
    }

    pub fn get_countless_copy(&self) -> Self {
        let mut copy = self.clone();
        copy.count = PredicateCount::default();
        copy
    }

    pub fn get_inverse_copy(&self) -> Self {
        let mut copy = self.clone();
        copy.inclusive = !copy.inclusive;
        copy
    }

    pub fn is_system(&self) -> bool {
        is_system_predicate(self.predicate_type)
    }

    /// `ToString`
    pub fn to_string(&self, with_count: bool) -> String {
        let mut base = self.base_string();

        if !self.inclusive {
            base = format!("- {base}");
        }

        if with_count {
            let suffix = self.count.get_suffix_string();
            if !suffix.is_empty() {
                base.push(' ');
                base.push_str(&suffix);
            }
        }

        base
    }

    fn base_string(&self) -> String {
        match (self.predicate_type, &self.value) {
            (PREDICATE_TYPE_TAG, PredicateValue::Text(text)) => text.clone(),
            (PREDICATE_TYPE_NAMESPACE, PredicateValue::Text(text)) => format!("{text}:*"),
            (PREDICATE_TYPE_WILDCARD, PredicateValue::Text(text)) => {
                format!("*{}*", text.trim_matches('*'))
            }
            (PREDICATE_TYPE_LABEL, PredicateValue::Text(text)) => text.clone(),
            (PREDICATE_TYPE_SYSTEM_EVERYTHING, _) => "everything".to_string(),
            (PREDICATE_TYPE_SYSTEM_INBOX, _) => "inbox".to_string(),
            (PREDICATE_TYPE_SYSTEM_ARCHIVE, _) => "archive".to_string(),
            (PREDICATE_TYPE_SYSTEM_UNTAGGED, _) => "untagged".to_string(),
            (PREDICATE_TYPE_SYSTEM_LOCAL, _) => "local".to_string(),
            (PREDICATE_TYPE_SYSTEM_NOT_LOCAL, _) => "not local".to_string(),
            (PREDICATE_TYPE_SYSTEM_DIMENSIONS, _) => "dimensions".to_string(),
            (PREDICATE_TYPE_SYSTEM_FILE_PROPERTIES, _) => "file properties".to_string(),
            (PREDICATE_TYPE_SYSTEM_SIMILAR_TO, _) => "similar files".to_string(),
            (PREDICATE_TYPE_SYSTEM_TIME, _) => "time".to_string(),
            (PREDICATE_TYPE_SYSTEM_URLS, _) => "urls".to_string(),
            (PREDICATE_TYPE_SYSTEM_NOTES, _) => "notes".to_string(),
            (PREDICATE_TYPE_OR_CONTAINER, PredicateValue::Or(predicates)) => {
                let inner = predicates
                    .iter()
                    .map(|predicate| predicate.to_string(false))
                    .collect::<Vec<_>>()
                    .join(" or ");
                format!("OR: {inner}")
            }
            (PREDICATE_TYPE_SYSTEM_LIMIT, PredicateValue::Number(number_test)) => {
                format!("limit {}", number_test.value as i64)
            }
            (predicate_type, PredicateValue::Number(number_test)) => {
                format!(
                    "{} {}",
                    predicate_type_string(predicate_type),
                    number_test.to_string()
                )
            }
            (PREDICATE_TYPE_SYSTEM_MIME, PredicateValue::Numbers(mimes)) => {
                let names = mimes
                    .iter()
                    .filter_map(|mime| Mime::try_from_int(*mime as u16))
                    .map(|mime| mime.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("mime: {names}")
            }
            (PREDICATE_TYPE_SYSTEM_HASH, PredicateValue::Hashes(hashes)) => {
                let hashes = hashes
                    .iter()
                    .map(|hash| hex_encode(hash))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("hash: {hashes}")
            }
            (predicate_type, PredicateValue::Service { .. }) => {
                predicate_type_string(predicate_type).to_string()
            }
            (predicate_type, _) => predicate_type_string(predicate_type).to_string(),
        }
    }
}

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
            self.predicate_type,
            serialise_value(self.predicate_type, &self.value),
            self.inclusive
        ])
    }

    fn initialise_from_serialisable_info(&mut self, info: &Value) -> HydrusResult<()> {
        let values = info
            .as_array()
            .ok_or_else(|| serialisation("Predicate info must be a list"))?;

        self.predicate_type = values.first().and_then(Value::as_i64).unwrap_or(0) as i32;
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

fn serialise_value(_predicate_type: i32, value: &PredicateValue) -> Value {
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

fn deserialise_value(predicate_type: i32, value: &Value) -> HydrusResult<PredicateValue> {
    if value.is_null() {
        return Ok(PredicateValue::None);
    }

    if matches!(
        predicate_type,
        PREDICATE_TYPE_TAG
            | PREDICATE_TYPE_NAMESPACE
            | PREDICATE_TYPE_WILDCARD
            | PREDICATE_TYPE_LABEL
    ) {
        return Ok(PredicateValue::Text(
            value.as_str().unwrap_or_default().to_string(),
        ));
    }

    if predicate_type == PREDICATE_TYPE_SYSTEM_FILE_SERVICE {
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
        PREDICATE_TYPE_SYSTEM_IMPORT_TIME
            | PREDICATE_TYPE_SYSTEM_MODIFIED_TIME
            | PREDICATE_TYPE_SYSTEM_ARCHIVED_TIME
            | PREDICATE_TYPE_SYSTEM_LAST_VIEWED_TIME
            | PREDICATE_TYPE_SYSTEM_TIME
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

    if predicate_type == PREDICATE_TYPE_SYSTEM_RATING {
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

    if predicate_type == PREDICATE_TYPE_SYSTEM_MIME {
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

    if predicate_type == PREDICATE_TYPE_SYSTEM_HASH {
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

    if predicate_type == PREDICATE_TYPE_OR_CONTAINER {
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

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }

    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_predicates_render_like_python() {
        let predicate = Predicate::tag("blue eyes");

        assert_eq!(predicate.to_string(true), "blue eyes");
        assert_eq!(predicate.get_namespace(), None);
        assert_eq!(
            predicate.get_texts_and_namespaces(),
            (Some("blue eyes".into()), None)
        );
    }

    #[test]
    fn namespace_predicates_append_wildcard() {
        assert_eq!(Predicate::namespace("creator").to_string(true), "creator:*");
    }

    #[test]
    fn negative_tag_predicates_get_a_prefix() {
        let mut predicate = Predicate::tag("blue eyes");
        predicate.set_inclusive(false);

        assert_eq!(predicate.to_string(true), "- blue eyes");
    }

    #[test]
    fn number_predicates_render_operator() {
        let predicate = Predicate::system_number(
            PREDICATE_TYPE_SYSTEM_WIDTH,
            NumberTest::new(
                crate::search::client_number_test::NUMBER_TEST_OPERATOR_GREATER_THAN,
                1000.0,
                None,
            ),
        );

        assert_eq!(predicate.to_string(true), "width > 1000");
    }

    #[test]
    fn serialisation_round_trips_tag_and_number() {
        for predicate in [
            Predicate::tag("blue eyes"),
            Predicate::system_number(PREDICATE_TYPE_SYSTEM_SIZE, NumberTest::new(2, 1024.0, None)),
        ] {
            let info = predicate.get_serialisable_info();
            let mut loaded = Predicate::tag("");

            loaded.initialise_from_serialisable_info(&info).unwrap();

            assert_eq!(loaded, predicate);
        }
    }
}
