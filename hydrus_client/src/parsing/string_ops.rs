//! Converter, joiner and processor from `hydrus/client/ClientStrings.py`.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use regex::Regex;

use crate::parsing::string_match::StringMatch;
use crate::parsing::string_tools::StringSplitter;

const URL_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
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

fn apply_conversion(text: &str, conversion: &Conversion) -> String {
    match conversion {
        Conversion::RemoveTextFromBeginning(count) => text.chars().skip(*count).collect(),
        Conversion::RemoveTextFromEnd(count) => {
            let length = text.chars().count();
            text.chars().take(length.saturating_sub(*count)).collect()
        }
        Conversion::ClipTextFromBeginning(count) => text.chars().take(*count).collect(),
        Conversion::ClipTextFromEnd(count) => {
            let length = text.chars().count();
            text.chars().skip(length.saturating_sub(*count)).collect()
        }
        Conversion::PrependText(prefix) => format!("{prefix}{text}"),
        Conversion::AppendText(suffix) => format!("{text}{suffix}"),
        Conversion::AppendRandom(character, count) => {
            format!("{text}{}", character.to_string().repeat(*count))
        }
        Conversion::Encode(encoding) => encode(text, *encoding),
        Conversion::Decode(encoding) => decode(text, *encoding),
        Conversion::Reverse => text.chars().rev().collect(),
        Conversion::RegexSub(pattern, replacement) => match Regex::new(pattern) {
            Ok(regex) => regex.replace_all(text, replacement.as_str()).into_owned(),
            Err(_) => text.to_string(),
        },
        Conversion::DateDecode(format) => chrono::NaiveDateTime::parse_from_str(text, format)
            .map(|date| date.and_utc().timestamp().to_string())
            .unwrap_or_else(|_| text.to_string()),
        Conversion::DateEncode(format) => text
            .parse::<i64>()
            .ok()
            .and_then(|value| chrono::DateTime::from_timestamp(value, 0))
            .map(|date| date.format(format).to_string())
            .unwrap_or_else(|| text.to_string()),
        Conversion::IntegerAddition(amount) => text
            .parse::<i64>()
            .map(|value| (value + amount).to_string())
            .unwrap_or_else(|_| text.to_string()),
    }
}

fn encode(text: &str, encoding: EncodingType) -> String {
    match encoding {
        EncodingType::UrlPercent => utf8_percent_encode(text, URL_ENCODE_SET).to_string(),
        EncodingType::UnicodeEscape => text
            .chars()
            .map(|character| {
                if character.is_ascii() {
                    character.to_string()
                } else {
                    format!("\\u{:04X}", character as u32)
                }
            })
            .collect(),
        EncodingType::HtmlEntities => text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;"),
        EncodingType::HexUtf8 => text
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        EncodingType::Base64Utf8 => base64_encode(text.as_bytes(), false, true),
        EncodingType::Base64UrlUtf8 => base64_encode(text.as_bytes(), true, false),
    }
}

fn decode(text: &str, encoding: EncodingType) -> String {
    match encoding {
        EncodingType::UrlPercent => percent_decode_str(text).decode_utf8_lossy().into_owned(),
        EncodingType::UnicodeEscape => decode_unicode_escapes(text),
        EncodingType::HtmlEntities => decode_html_entities(text),
        EncodingType::HexUtf8 => decode_hex(text),
        EncodingType::Base64Utf8 => base64_decode(text, false),
        EncodingType::Base64UrlUtf8 => base64_decode(text, true),
    }
}

fn decode_unicode_escapes(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(character) = chars.next() {
        if character == '\\' && chars.peek() == Some(&'u') {
            chars.next();
            let hex: String = chars.by_ref().take(4).collect();

            if let Ok(value) = u32::from_str_radix(&hex, 16) {
                if let Some(decoded) = char::from_u32(value) {
                    result.push(decoded);
                    continue;
                }
            }

            result.push_str("\\u");
            result.push_str(&hex);
        } else {
            result.push(character);
        }
    }

    result
}

pub fn decode_html_entities(text: &str) -> String {
    let mut result = text
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");

    let regex = Regex::new(r"&#(\d+);").expect("static regex");

    while let Some(captures) = regex.captures(&result) {
        let whole = captures.get(0).expect("capture zero");
        let value = captures[1].parse::<u32>().unwrap_or(0);

        match char::from_u32(value) {
            Some(decoded) => {
                result.replace_range(whole.range(), &decoded.to_string());
            }
            None => {
                result.replace_range(whole.range(), "");
            }
        }
    }

    result
}

fn decode_hex(text: &str) -> String {
    let bytes: Vec<u8> = text
        .as_bytes()
        .chunks(2)
        .filter_map(|chunk| {
            let hex = std::str::from_utf8(chunk).ok()?;
            u8::from_str_radix(hex, 16).ok()
        })
        .collect();

    String::from_utf8_lossy(&bytes).into_owned()
}

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(bytes: &[u8], url_safe: bool, pad: bool) -> String {
    let mut encoded = String::new();

    for chunk in bytes.chunks(3) {
        let value = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;

        let length = encoded.len();

        for index in 0..4 {
            let char_index = ((value >> (18 - index * 6)) & 0x3F) as usize;
            encoded.push(BASE64_ALPHABET[char_index] as char);
        }

        match chunk.len() {
            1 => {
                encoded.truncate(length + 2);
                if pad {
                    encoded.push_str("==");
                }
            }
            2 => {
                encoded.truncate(length + 3);
                if pad {
                    encoded.push('=');
                }
            }
            _ => {}
        }
    }

    if url_safe {
        encoded = encoded.replace('+', "-").replace('/', "_");
    }

    encoded
}

fn base64_decode(text: &str, url_safe: bool) -> String {
    let cleaned: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace() && *byte != b'=')
        .map(|byte| match byte {
            b'-' => b'+',
            b'_' => b'/',
            other => other,
        })
        .collect();

    let mut bytes = Vec::new();

    for chunk in cleaned.chunks(4) {
        if chunk.len() == 1 {
            break;
        }

        let mut value = 0u32;

        for (index, byte) in chunk.iter().enumerate() {
            let position = BASE64_ALPHABET
                .iter()
                .position(|candidate| candidate == byte)
                .unwrap_or(0) as u32;

            value |= position << (18 - index * 6);
        }

        bytes.push((value >> 16) as u8);

        if chunk.len() > 2 {
            bytes.push((value >> 8) as u8);
        }
        if chunk.len() > 3 {
            bytes.push(value as u8);
        }
    }

    let _ = url_safe;

    String::from_utf8_lossy(&bytes).into_owned()
}

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
