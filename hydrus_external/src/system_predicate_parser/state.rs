//! Mutable parse state shared by the parser stages.

use super::types::{Operator, Predicate, Unit, ValueData};

pub(crate) struct ParseState {
    pub original_text: String,
    pub original_text_lower: String,
    pub subtag_text_lower: String,
    pub text_remainder: String,
    pub predicate: Option<Predicate>,
    pub operator: Option<Operator>,
    pub value: Option<ValueData>,
    pub unit: Option<Unit>,
}

impl ParseState {
    pub fn new(text: &str) -> Self {
        let original_text = text.trim().to_string();
        let original_text_lower = original_text.to_lowercase();
        let subtag_text_lower = original_text_lower
            .strip_prefix("system:")
            .unwrap_or(&original_text_lower)
            .to_string();

        Self {
            original_text,
            original_text_lower,
            text_remainder: subtag_text_lower.clone(),
            subtag_text_lower,
            predicate: None,
            operator: None,
            value: None,
            unit: None,
        }
    }
}
