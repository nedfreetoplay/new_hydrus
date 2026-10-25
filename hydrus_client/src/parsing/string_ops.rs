//! Converter, joiner and processor from `hydrus/client/ClientStrings.py`.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC};

use crate::parsing::string_match::StringMatch;
use crate::parsing::string_tools::StringSplitter;

pub(super) const URL_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conversion {
    RemoveTextFromBeginning(usize),
    RemoveTextFromEnd(usize),
    ClipTextFromBeginning(usize),
    ClipTextFromEnd(usize),
    PrependText(String),
    AppendText(String),
    AppendRandom(char, usize),
    Encode(EncodingType),
    Decode(EncodingType),
    Reverse,
    RegexSub(String, String),
    DateDecode(String),
    DateEncode(String),
    IntegerAddition(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingType {
    UrlPercent,
    UnicodeEscape,
    HtmlEntities,
    HexUtf8,
    Base64Utf8,
    Base64UrlUtf8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StringConverter {
    pub conversions: Vec<Conversion>,
}

impl StringConverter {
    pub fn new(conversions: Vec<Conversion>) -> Self {
        Self { conversions }
    }

    pub fn convert(&self, text: &str) -> String {
        let mut value = text.to_string();

        for conversion in &self.conversions {
            value = apply_conversion(&value, conversion);
        }

        value
    }
}

mod base64;
mod encodings;
use encodings::apply_conversion;
pub use encodings::decode_html_entities;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringJoiner {
    pub joiner: String,
    pub join_tuple_size: Option<usize>,
}

impl StringJoiner {
    pub fn new(joiner: impl Into<String>, join_tuple_size: Option<usize>) -> Self {
        Self {
            joiner: unescape_python(joiner.into()),
            join_tuple_size,
        }
    }

    pub fn join(&self, strings: &[String]) -> Vec<String> {
        match self.join_tuple_size {
            Some(size) if size > 1 => strings
                .chunks(size)
                .filter(|chunk| chunk.len() == size)
                .map(|chunk| chunk.join(&self.joiner))
                .collect(),
            _ => vec![strings.join(&self.joiner)],
        }
    }

    pub fn to_string(&self) -> String {
        match self.join_tuple_size {
            Some(size) if size > 1 => format!(
                "joining every {size} strings using \"{}\"",
                self.joiner.escape_debug()
            ),
            _ => format!(
                "joining all strings using \"{}\"",
                self.joiner.escape_debug()
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessingStep {
    Splitter(StringSplitter),
    Match(StringMatch),
    Converter(StringConverter),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StringProcessor {
    pub steps: Vec<ProcessingStep>,
}

impl StringProcessor {
    pub fn new(steps: Vec<ProcessingStep>) -> Self {
        Self { steps }
    }

    pub fn process_strings(&self, strings: impl IntoIterator<Item = String>) -> Vec<String> {
        let mut current: Vec<String> = strings.into_iter().collect();

        for step in &self.steps {
            let mut next = Vec::new();

            for text in current {
                match step {
                    ProcessingStep::Splitter(splitter) => next.extend(splitter.split(&text)),
                    ProcessingStep::Match(matcher) => {
                        if matcher.matches(&text) {
                            next.push(text);
                        }
                    }
                    ProcessingStep::Converter(converter) => next.push(converter.convert(&text)),
                }
            }

            current = next;
        }

        current
    }
}

fn unescape_python(value: String) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\\", "\\")
}

#[cfg(test)]
mod tests;
