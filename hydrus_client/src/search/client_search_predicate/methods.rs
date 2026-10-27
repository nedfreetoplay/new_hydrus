//! Predicate helper methods.

use hydrus_core::hydrus_constants::Mime;

use crate::search::client_number_test::NumberTest;
use crate::search::predicate_constants::*;

use super::hex::hex_encode;
use super::{Predicate, PredicateCount, PredicateValue};

impl Predicate {
    pub fn new(predicate_type: PredicateType, value: PredicateValue) -> Self {
        Self {
            predicate_type,
            value,
            inclusive: true,
            count: PredicateCount::default(),
        }
    }

    pub fn tag(tag: impl Into<String>) -> Self {
        Self::new(PredicateType::Tag, PredicateValue::Text(tag.into()))
    }

    pub fn namespace(namespace: impl Into<String>) -> Self {
        Self::new(
            PredicateType::Namespace,
            PredicateValue::Text(namespace.into()),
        )
    }

    pub fn system_number(predicate_type: PredicateType, number_test: NumberTest) -> Self {
        Self::new(predicate_type, PredicateValue::Number(number_test))
    }

    pub fn get_type(&self) -> PredicateType {
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
            (PredicateValue::Text(text), PredicateType::Tag | PredicateType::Parent) => {
                match text.split_once(':') {
                    Some((namespace, subtag)) => {
                        (Some(subtag.to_string()), Some(namespace.to_string()))
                    }
                    None => (Some(text.clone()), None),
                }
            }
            (PredicateValue::Text(text), PredicateType::Namespace) => (None, Some(text.clone())),
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
            (PredicateType::Tag, PredicateValue::Text(text)) => text.clone(),
            (PredicateType::Namespace, PredicateValue::Text(text)) => format!("{text}:*"),
            (PredicateType::Wildcard, PredicateValue::Text(text)) => {
                format!("*{}*", text.trim_matches('*'))
            }
            (PredicateType::Label, PredicateValue::Text(text)) => text.clone(),
            (PredicateType::SystemEverything, _) => "everything".to_string(),
            (PredicateType::SystemInbox, _) => "inbox".to_string(),
            (PredicateType::SystemArchive, _) => "archive".to_string(),
            (PredicateType::SystemUntagged, _) => "untagged".to_string(),
            (PredicateType::SystemLocal, _) => "local".to_string(),
            (PredicateType::SystemNotLocal, _) => "not local".to_string(),
            (PredicateType::SystemDimensions, _) => "dimensions".to_string(),
            (PredicateType::SystemFileProperties, _) => "file properties".to_string(),
            (
                PredicateType::SystemSimilarTo,
                PredicateValue::Similar {
                    hashes, distance, ..
                },
            ) => {
                format!("similar to {} files (distance {distance})", hashes.len())
            }
            (
                PredicateType::SystemFileRelationshipsCount,
                PredicateValue::Relationships { number, kind },
            ) => {
                format!("{kind} relationships {}", number.to_string())
            }
            (PredicateType::SystemSimilarTo, _) => "similar files".to_string(),
            (PredicateType::SystemTime, _) => "time".to_string(),
            (PredicateType::SystemUrls, _) => "urls".to_string(),
            (PredicateType::SystemKnownUrls, PredicateValue::KnownUrl { rule_type, rule }) => {
                format!("known url: {rule_type} = {rule}")
            }
            (
                PredicateType::SystemFileViewingStats,
                PredicateValue::ViewingStats {
                    canvas_types,
                    operator,
                    value,
                    viewtime,
                },
            ) => {
                let subject = if *viewtime { "viewtime" } else { "views" };
                let canvas = canvas_types.join("+");

                format!(
                    "{canvas} {subject} {} {}",
                    crate::search::client_number_test::operator_string(*operator),
                    value
                )
            }
            (PredicateType::SystemNotes, _) => "notes".to_string(),
            (PredicateType::OrContainer, PredicateValue::Or(predicates)) => {
                let inner = predicates
                    .iter()
                    .map(|predicate| predicate.to_string(false))
                    .collect::<Vec<_>>()
                    .join(" or ");
                format!("OR: {inner}")
            }
            (PredicateType::SystemLimit, PredicateValue::Number(number_test)) => {
                format!("limit {}", number_test.value as i64)
            }
            (predicate_type, PredicateValue::Number(number_test)) => {
                format!("{} {}", predicate_type, number_test.to_string())
            }
            (PredicateType::SystemMime, PredicateValue::Numbers(mimes)) => {
                let names = mimes
                    .iter()
                    .filter_map(|mime| Mime::try_from_int(*mime as u16))
                    .map(|mime| mime.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("mime: {names}")
            }
            (PredicateType::SystemHash, PredicateValue::Hashes(hashes)) => {
                let hashes = hashes
                    .iter()
                    .map(|hash| hex_encode(hash))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("hash: {hashes}")
            }
            (predicate_type, PredicateValue::Service { .. }) => predicate_type.to_string(),
            (predicate_type, _) => predicate_type.to_string(),
        }
    }
}
