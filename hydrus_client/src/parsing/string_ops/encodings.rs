//! Strings encoders, decoders and conversion steps.

use percent_encoding::{percent_decode_str, utf8_percent_encode};
use regex::Regex;

use super::{Conversion, EncodingType, URL_ENCODE_SET};

pub(super) fn apply_conversion(text: &str, conversion: &Conversion) -> String {
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

pub(crate) fn encode(text: &str, encoding: EncodingType) -> String {
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

pub(crate) fn decode(text: &str, encoding: EncodingType) -> String {
    match encoding {
        EncodingType::UrlPercent => percent_decode_str(text).decode_utf8_lossy().into_owned(),
        EncodingType::UnicodeEscape => decode_unicode_escapes(text),
        EncodingType::HtmlEntities => decode_html_entities(text),
        EncodingType::HexUtf8 => decode_hex(text),
        EncodingType::Base64Utf8 => base64_decode(text, false),
        EncodingType::Base64UrlUtf8 => base64_decode(text, true),
    }
}

pub(crate) fn decode_unicode_escapes(text: &str) -> String {
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

pub(crate) fn decode_hex(text: &str) -> String {
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

use super::base64::{base64_decode, base64_encode};
