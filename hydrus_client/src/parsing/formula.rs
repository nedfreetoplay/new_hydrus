//! Parse formulas from `hydrus/client/parsing/ClientParsing.py`.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::parsing::string_match::StringMatch;
use crate::parsing::string_ops::StringProcessor;

pub type ParsingContext = BTreeMap<String, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonContent {
    String,
    Json,
    DictKeys,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonRule {
    DictKey(StringMatch),
    IndexedItem(usize),
    AllItems,
    TestStringItems(StringMatch),
    Ascend(usize),
}

pub trait ParseFormula {
    fn parse(&self, context: &ParsingContext, text: &str, collapse_newlines: bool) -> Vec<String>;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonFormula {
    pub name: String,
    pub rules: Vec<JsonRule>,
    pub content: JsonContent,
    pub processor: StringProcessor,
}

impl JsonFormula {
    pub fn new(rules: Vec<JsonRule>, content: JsonContent) -> Self {
        Self {
            name: String::new(),
            rules,
            content,
            processor: StringProcessor::default(),
        }
    }

    fn raw_texts(&self, text: &str) -> Vec<String> {
        let Ok(root) = serde_json::from_str::<Value>(text) else {
            return Vec::new();
        };

        let mut nodes: Vec<(Value, Vec<Value>)> = vec![(root, Vec::new())];

        for rule in &self.rules {
            let mut next = Vec::new();

            for (node, stack) in &nodes {
                let mut next_stack = stack.clone();
                next_stack.push(node.clone());

                match rule {
                    JsonRule::AllItems => match node {
                        Value::Array(items) => {
                            for item in items {
                                next.push((item.clone(), next_stack.clone()));
                            }
                        }
                        Value::Object(map) => {
                            let mut pairs: Vec<_> = map.iter().collect();
                            pairs.sort_by(|left, right| left.0.cmp(right.0));

                            for (_, value) in pairs {
                                next.push((value.clone(), next_stack.clone()));
                            }
                        }
                        _ => {}
                    },
                    JsonRule::IndexedItem(index) => match node {
                        Value::Array(items) => {
                            if let Some(item) = items.get(*index) {
                                next.push((item.clone(), next_stack));
                            }
                        }
                        Value::Object(map) => {
                            let mut keys: Vec<&String> = map.keys().collect();
                            keys.sort_by_key(|key| human_key(key));

                            if let Some(key) = keys.get(*index) {
                                if let Some(value) = map.get(*key) {
                                    next.push((value.clone(), next_stack));
                                }
                            }
                        }
                        _ => {}
                    },
                    JsonRule::DictKey(matcher) => {
                        if let Value::Object(map) = node {
                            let mut pairs: Vec<_> = map.iter().collect();
                            pairs.sort_by(|left, right| left.0.cmp(right.0));

                            for (key, value) in pairs {
                                if matcher.matches(key) {
                                    next.push((value.clone(), next_stack.clone()));
                                }
                            }
                        }
                    }
                    JsonRule::TestStringItems(matcher) => {
                        if let Some(text) = scalar_to_string(node) {
                            if matcher.matches(&text) {
                                next.push((node.clone(), stack.clone()));
                            }
                        }
                    }
                    JsonRule::Ascend(steps) => {
                        if stack.len() >= *steps {
                            let ancestor = stack[stack.len() - *steps].clone();
                            let ancestor_stack = stack[..stack.len() - *steps].to_vec();
                            next.push((ancestor, ancestor_stack));
                        }
                    }
                }
            }

            nodes = next;
        }

        let mut texts = Vec::new();

        for (node, _) in nodes {
            match self.content {
                JsonContent::String => {
                    if let Some(text) = scalar_to_string(&node) {
                        texts.push(text);
                    }
                }
                JsonContent::Json => {
                    texts.push(serde_json::to_string(&node).unwrap_or_default());
                }
                JsonContent::DictKeys => {
                    if let Value::Object(map) = node {
                        let mut keys: Vec<_> = map.keys().cloned().collect();
                        keys.sort();
                        texts.extend(keys);
                    }
                }
            }
        }

        texts
    }
}

impl ParseFormula for JsonFormula {
    fn parse(&self, _context: &ParsingContext, text: &str, collapse_newlines: bool) -> Vec<String> {
        process_texts(self.raw_texts(text), &self.processor, collapse_newlines)
    }

    fn name(&self) -> &str {
        &self.name
    }
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

fn scalar_to_string(node: &Value) -> Option<String> {
    match node {
        Value::Null => None,
        Value::String(value) => Some(value.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn human_key(value: &String) -> Vec<(u8, i64, String)> {
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
