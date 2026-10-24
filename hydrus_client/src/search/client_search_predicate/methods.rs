//! Predicate helper methods.

use hydrus_core::hydrus_constants::Mime;

use crate::search::client_number_test::NumberTest;
use crate::search::predicate_constants::*;

use super::hex::hex_encode;
use super::{Predicate, PredicateCount, PredicateValue};

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
