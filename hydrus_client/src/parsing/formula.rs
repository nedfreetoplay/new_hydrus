//! Parse formulas from `hydrus/client/parsing/ClientParsing.py`.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::parsing::string_match::StringMatch;
use crate::parsing::string_ops::StringProcessor;

pub type ParsingContext = BTreeMap<String, String>;

mod json;
pub use json::{JsonContent, JsonFormula, JsonRule};

pub trait ParseFormula {
    fn parse(&self, context: &ParsingContext, text: &str, collapse_newlines: bool) -> Vec<String>;
    fn name(&self) -> &str;
}

/// Shared cleanup and string processing used by every formula type.
pub fn finish_texts(texts: Vec<String>, collapse_newlines: bool) -> Vec<String> {
    let processed: Vec<String> = texts
        .into_iter()
        .map(|text| {
            let text = if collapse_newlines {
                text.replace(['\r', '\n'], "")
            } else {
                text.trim().to_string()
            };

            text.trim().to_string()
        })
        .filter(|text| !text.is_empty())
        .collect();

    processed
}

pub fn process_texts(
    texts: Vec<String>,
    processor: &StringProcessor,
    collapse_newlines: bool,
) -> Vec<String> {
    processor.process_strings(finish_texts(texts, collapse_newlines))
}

pub(super) fn scalar_to_string(node: &Value) -> Option<String> {
    match node {
        Value::Null => None,
        Value::String(value) => Some(value.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

pub(super) fn human_key(value: &String) -> Vec<(u8, i64, String)> {
    let mut parts = Vec::new();
    let mut digits = String::new();
    let mut letters = String::new();

    for character in value.chars() {
        if character.is_ascii_digit() {
            if !letters.is_empty() {
                parts.push((0, 0, std::mem::take(&mut letters)));
            }
            digits.push(character);
        } else {
            if !digits.is_empty() {
                parts.push((1, digits.parse().unwrap_or(0), String::new()));
                digits.clear();
            }
            letters.push(character);
        }
    }

    if !digits.is_empty() {
        parts.push((1, digits.parse().unwrap_or(0), String::new()));
    }
    if !letters.is_empty() {
        parts.push((0, 0, letters));
    }

    parts
}

#[derive(Default)]
pub struct NestedFormula {
    pub main: Option<Box<dyn ParseFormula>>,
    pub sub: Option<Box<dyn ParseFormula>>,
}

impl NestedFormula {
    pub fn new(main: Box<dyn ParseFormula>, sub: Box<dyn ParseFormula>) -> Self {
        Self {
            main: Some(main),
            sub: Some(sub),
        }
    }
}

impl ParseFormula for NestedFormula {
    fn parse(&self, context: &ParsingContext, text: &str, collapse_newlines: bool) -> Vec<String> {
        let (Some(main), Some(sub)) = (&self.main, &self.sub) else {
            return Vec::new();
        };

        let mut results = Vec::new();

        for parsed in main.parse(context, text, collapse_newlines) {
            results.extend(sub.parse(context, &parsed, true));
        }

        results
    }

    fn name(&self) -> &str {
        "nested formula"
    }
}

#[derive(Default)]
pub struct ZipperFormula {
    pub formulae: Vec<Box<dyn ParseFormula>>,
    pub sub_phrase: String,
}

impl ZipperFormula {
    pub fn new(formulae: Vec<Box<dyn ParseFormula>>, sub_phrase: impl Into<String>) -> Self {
        Self {
            formulae,
            sub_phrase: sub_phrase.into(),
        }
    }
}

impl ParseFormula for ZipperFormula {
    fn parse(&self, context: &ParsingContext, text: &str, collapse_newlines: bool) -> Vec<String> {
        let mut results: Vec<String> = Vec::new();

        for (index, formula) in self.formulae.iter().enumerate() {
            let Some(value) = formula
                .parse(context, text, collapse_newlines)
                .into_iter()
                .next()
            else {
                continue;
            };

            let marker = format!("\\{}", index + 1);

            while results.len() <= index {
                results.push(String::new());
            }

            results[index] = value;
        }

        if results.len() != self.formulae.len() {
            return Vec::new();
        }

        vec![self.sub_phrase.clone()]
            .into_iter()
            .map(|phrase| {
                let mut phrase = phrase;

                for (index, value) in results.iter().enumerate() {
                    phrase = phrase.replace(&format!("\\{}", index + 1), value);
                }

                phrase
            })
            .collect()
    }

    fn name(&self) -> &str {
        "zipper formula"
    }
}

#[cfg(test)]
mod tests;
