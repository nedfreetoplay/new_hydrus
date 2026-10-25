//! JSON parsing formula.

use serde_json::Value;

use crate::parsing::string_match::StringMatch;
use crate::parsing::string_ops::StringProcessor;

use super::{ParseFormula, ParsingContext, human_key, process_texts, scalar_to_string};

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
