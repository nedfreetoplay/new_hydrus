//! Rust port of `hydrus/core/HydrusText.py`.
//!
//! Most of this is regexes and text munging. The two Python lookahead regexes
//! are exposed as functions because Rust's `regex` crate has no lookaround.

use std::sync::LazyLock;

use regex::Regex;

use crate::hydrus_constants as HC;
use crate::hydrus_numbers;

pub static RE_ONE_OR_MORE_WHITESPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// `^(-|system:)+`
pub static RE_LEADING_GARBAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(-|system:)+").unwrap());

/// `^:[^:]+$` -- Python's `re_leading_single_colon_and_no_more_colons`
/// (`^:(?=[^:]+$)`) as a plain match.
pub static RE_LEADING_SINGLE_COLON_AND_NO_MORE_COLONS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^:[^:]+$").unwrap());

/// Python's `re_leading_single_colon_and_later_colon`
/// (`^:(?=[^:]+:[^:]+$)`).
pub static RE_LEADING_SINGLE_COLON_AND_LATER_COLON: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^:[^:]+:[^:]+$").unwrap());

pub static RE_LEADING_BYTE_ORDER_MARK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("^{}", HC::UNICODE_BYTE_ORDER_MARK)).unwrap());

pub static RE_UNDESIRED_CONTROL_CHARACTERS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"[\x{0000}-\x{001F}\x{007F}-\x{009F}\x{200B}\x{200E}\x{200F}\x{202A}-\x{202E}\x{2066}-\x{2069}\x{FEFF}\x{E000}-\x{F8FF}\x{F0000}-\x{FFFFD}\x{100000}-\x{10FFFD}]",
    )
    .unwrap()
});

pub static RE_OOPS_ALL_ZERO_WIDTH_JOINERS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\x{200C}\x{200D}]+$").unwrap());

pub static RE_ZERO_WIDTH_JOINERS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\x{200C}\x{200D}]").unwrap());

pub static RE_THIS_IS_ALL_LATIN_AND_ZERO_WIDTH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\x{0020}-\x{007E}\x{00A0}-\x{024F}\x{200C}\x{200D}]+$").unwrap()
});

pub static RE_LOOKS_LIKE_HANGUL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\x{1100}-\x{11FF}\x{AC00}-\x{D7AF}]").unwrap());

pub const HANGUL_FILLER_CHARACTER: char = '\u{3164}';

pub const HYDRUS_NOTE_NEWLINE: &str = "\n";

pub const NULL_CHARACTER: char = '\x00';

/// Python: `re_leading_single_colon_and_no_more_colons.match( tag ) is not None`
pub fn leading_single_colon_and_no_more_colons(text: &str) -> bool {
    RE_LEADING_SINGLE_COLON_AND_NO_MORE_COLONS.is_match(text)
}

/// Python: `re_leading_single_colon_and_later_colon.match( tag ) is not None`
pub fn leading_single_colon_and_later_colon(text: &str) -> bool {
    RE_LEADING_SINGLE_COLON_AND_LATER_COLON.is_match(text)
}

/// `cleanse_import_text`. Python re-encodes lone utf-16 surrogates; Rust
/// strings cannot contain them, so this is a no-op unless the caller still
/// has raw bytes (see [`cleanse_import_bytes`]).
pub fn cleanse_import_text(text: &str) -> String {
    // Lone surrogates are impossible in a Rust `str`. Normalise nothing.
    text.to_string()
}

/// Decode WTF-16 style data (odd surrogate splitting) and drop any lone
/// surrogate replacement characters.
pub fn cleanse_import_bytes(data: &[u8]) -> String {
    let (cow, _, _) = encoding_rs::UTF_16LE.decode(data);

    // Strip U+FFFD that came from lone surrogates at the edges.
    cow.trim_matches(HC::UNICODE_REPLACEMENT_CHARACTER.chars().next().unwrap())
        .to_string()
}

pub fn cleanse_import_texts<I, S>(texts: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    texts
        .into_iter()
        .map(|text| cleanse_import_text(text.as_ref()))
        .collect()
}

/// `clean_note_text`
pub fn clean_note_text(t: &str) -> String {
    let t = t.trim();

    let lines: Vec<&str> = t.lines().map(str::trim).collect();

    let mut t = lines.join(HYDRUS_NOTE_NEWLINE);

    let double_newline = format!("{HYDRUS_NOTE_NEWLINE}{HYDRUS_NOTE_NEWLINE}");
    let mut triple_newline = format!("{double_newline}{HYDRUS_NOTE_NEWLINE}");

    while t.contains(&triple_newline) {
        t = t.replace(&triple_newline, &double_newline);
        triple_newline = format!("{double_newline}{HYDRUS_NOTE_NEWLINE}");
    }

    t
}

/// `convert_many_strings_to_nice_insertable_human_summary`
pub fn convert_many_strings_to_nice_insertable_human_summary<S: AsRef<str>>(
    texts: &[S],
    do_sort: bool,
    no_trailing_whitespace: bool,
) -> String {
    let mut texts_list: Vec<String> = texts.iter().map(|t| t.as_ref().to_string()).collect();

    if do_sort {
        human_text_sort(&mut texts_list);
    }

    if texts_list.len() == 1 {
        if no_trailing_whitespace {
            format!(" \"{}\"", texts_list[0])
        } else {
            format!(" \"{}\" ", texts_list[0])
        }
    } else {
        let t = if texts_list.len() <= 4 {
            texts_list.join("\n")
        } else {
            const LINE_NO_LONGER_THAN: usize = 64;
            const NUM_LINES_LIMIT: usize = 24;

            let mut lines: Vec<String> = Vec::new();
            let mut line_under_construction = String::new();

            let mut texts_to_do: Vec<String> = texts_list.clone();

            while !texts_to_do.is_empty() {
                let text = texts_to_do.remove(0);

                if line_under_construction.is_empty() {
                    line_under_construction = text;
                } else {
                    let potential_next_line = format!("{line_under_construction}, {text}");

                    if potential_next_line.chars().count() > LINE_NO_LONGER_THAN {
                        lines.push(line_under_construction);

                        if lines.len() >= NUM_LINES_LIMIT {
                            line_under_construction = String::new();
                            texts_to_do.insert(0, text);

                            lines.push(format!(
                                "and {} others",
                                hydrus_numbers::to_human_int(texts_to_do.len() as i128)
                            ));

                            break;
                        } else {
                            line_under_construction = text;
                        }
                    } else {
                        line_under_construction = potential_next_line;
                    }
                }
            }

            if !line_under_construction.is_empty() {
                lines.push(line_under_construction);
            }

            lines.join("\n")
        };

        if no_trailing_whitespace {
            format!("\n\n{t}")
        } else {
            format!("\n\n{t}\n\n")
        }
    }
}

/// `convert_many_strings_to_nice_insertable_human_summary_single_line`
pub fn convert_many_strings_to_nice_insertable_human_summary_single_line<S: AsRef<str>>(
    texts: &[S],
    collective_description_noun: &str,
    do_sort: bool,
) -> String {
    if texts.is_empty() {
        return format!("0(?) {collective_description_noun}");
    }

    let mut texts_list: Vec<String> = texts.iter().map(|t| t.as_ref().to_string()).collect();

    if do_sort {
        human_text_sort(&mut texts_list);
    }

    const LINE_NO_LONGER_THAN: usize = 48;

    if texts_list.len() == 1 {
        let text = &texts_list[0];

        if text.chars().count() + 2 > LINE_NO_LONGER_THAN {
            return format!("1 {collective_description_noun}");
        }

        return format!("\"{text}\"");
    }

    let full_result = texts_list
        .iter()
        .map(|text| format!("\"{text}\""))
        .collect::<Vec<_>>()
        .join(", ");

    if full_result.chars().count() <= LINE_NO_LONGER_THAN {
        return full_result;
    }

    let first_text = &texts_list[0];
    let num_texts = texts_list.len();

    let leading_example_result = format!(
        "\"{}\" & {} other {}",
        first_text,
        hydrus_numbers::to_human_int(num_texts as i128 - 1),
        collective_description_noun
    );

    if leading_example_result.chars().count() <= LINE_NO_LONGER_THAN {
        leading_example_result
    } else {
        format!(
            "{} {}",
            hydrus_numbers::to_human_int(num_texts as i128),
            collective_description_noun
        )
    }
}

/// `hex_filter`
pub fn hex_filter(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect()
}

/// `deserialise_newlined_texts`
pub fn deserialise_newlined_texts(text: &str) -> Vec<String> {
    text.lines()
        .map(strip_io_input_line)
        .filter(|line| !line.is_empty())
        .collect()
}

/// `elide_text`
pub fn elide_text(text: &str, max_length: usize, elide_center: bool) -> String {
    if text.chars().count() > max_length {
        if elide_center {
            let center_end_chars = (max_length / 8).max(2);

            let head: String = text
                .chars()
                .take(max_length.saturating_sub(1 + center_end_chars))
                .collect();

            let tail: String = {
                let total = text.chars().count();
                text.chars()
                    .skip(total.saturating_sub(center_end_chars))
                    .collect()
            };

            format!("{head}{}{tail}", HC::UNICODE_ELLIPSIS)
        } else {
            let head: String = text.chars().take(max_length.saturating_sub(1)).collect();

            format!("{head}{}", HC::UNICODE_ELLIPSIS)
        }
    } else {
        text.to_string()
    }
}

/// `get_first_line`
pub fn get_first_line(text: Option<&str>) -> String {
    match text {
        None => "unknown".to_string(),
        Some(text) => text.lines().next().unwrap_or("").to_string(),
    }
}

/// `get_first_line_summary`
pub fn get_first_line_summary(text: Option<&str>) -> String {
    let Some(text) = text else {
        return "unknown".to_string();
    };

    let lines: Vec<&str> = text.lines().collect();

    if lines.len() > 1 {
        format!(
            "{}{} (+{} lines)",
            lines[0],
            HC::UNICODE_ELLIPSIS,
            hydrus_numbers::to_human_int(lines.len() as i128 - 1)
        )
    } else {
        text.to_string()
    }
}

/// `human_text_sort_key`: split into `( text, number )` chunks so that
/// `Page 2` sorts before `Page 19`.
pub fn human_text_sort_key(text: &str) -> Vec<(String, u64)> {
    let casefolded = text.to_lowercase();

    let mut result: Vec<(String, u64)> = Vec::new();
    let mut buffer = String::new();

    let mut digit_run = String::new();
    let mut in_digits = false;

    for ch in casefolded.chars() {
        if ch.is_ascii_digit() {
            if !in_digits {
                result.push((buffer.clone(), 0));
                buffer.clear();
                in_digits = true;
            }

            digit_run.push(ch);
        } else {
            if in_digits {
                let value = digit_run.parse::<u64>().unwrap_or(u64::MAX);
                result.push((String::new(), value));
                digit_run.clear();
                in_digits = false;
            }

            buffer.push(ch);
        }
    }

    if in_digits {
        let value = digit_run.parse::<u64>().unwrap_or(u64::MAX);
        result.push((String::new(), value));
    } else {
        result.push((buffer, 0));
    }

    result
}

/// `human_text_sort`
pub fn human_text_sort<S: AsMut<str> + AsRef<str>>(texts: &mut [S]) {
    texts.sort_by(|a, b| human_text_sort_key(a.as_ref()).cmp(&human_text_sort_key(b.as_ref())));
}

/// `human_text_sort` over owned strings.
pub fn human_text_sort_strings(texts: &mut [String]) {
    texts.sort_by(|a, b| human_text_sort_key(a).cmp(&human_text_sort_key(b)));
}

/// `looks_like_html`
pub fn looks_like_html(file_data: &[u8]) -> bool {
    [
        &b"<html"[..],
        b"<HTML",
        b"<!DOCTYPE html",
        b"<!DOCTYPE HTML",
    ]
    .iter()
    .any(|needle| memmem(file_data, needle))
}

pub fn looks_like_html_str(file_data: &str) -> bool {
    ["<html", "<HTML", "<!DOCTYPE html", "<!DOCTYPE HTML"]
        .iter()
        .any(|needle| file_data.contains(needle))
}

/// `looks_like_svg`
pub fn looks_like_svg(file_data: &[u8]) -> bool {
    [&b"<svg"[..], b"<SVG", b"<!DOCTYPE svg", b"<!DOCTYPE SVG"]
        .iter()
        .any(|needle| memmem(file_data, needle))
}

pub fn looks_like_svg_str(file_data: &str) -> bool {
    ["<svg", "<SVG", "<!DOCTYPE svg", "<!DOCTYPE SVG"]
        .iter()
        .any(|needle| file_data.contains(needle))
}

fn memmem(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

/// `looks_like_json`
pub fn looks_like_json(file_data: &[u8]) -> bool {
    match std::str::from_utf8(file_data) {
        Ok(text) => serde_json::from_str::<serde_json::Value>(text).is_ok(),
        Err(_) => false,
    }
}

pub fn looks_like_json_str(file_data: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(file_data).is_ok()
}

/// Encodings Python treats as untrustworthy browser defaults.
pub const DEFAULT_WEB_ENCODINGS: &[&str] = &["ISO-8859-1", "Windows-1252"];

/// `non_failing_unicode_decode`. Returns `( text, encoding )`.
pub fn non_failing_unicode_decode(
    data: &[u8],
    encoding: Option<&str>,
    trust_the_encoding: bool,
) -> (String, String) {
    if trust_the_encoding {
        if let Some(label) = encoding {
            if let Some(enc) = encoding_rs::Encoding::for_label(label.as_bytes()) {
                let (text, _) = enc.decode_with_bom_removal(data);
                return (strip_null(text.into_owned()), enc.name().to_string());
            }
        }
    }

    let bogus_default = match encoding {
        Some(label) => DEFAULT_WEB_ENCODINGS
            .iter()
            .any(|default| default.eq_ignore_ascii_case(label)),
        None => true,
    };

    if !bogus_default {
        if let Some(enc) =
            encoding.and_then(|label| encoding_rs::Encoding::for_label(label.as_bytes()))
        {
            let (text, _, had_errors) = enc.decode(data);

            if !had_errors {
                return (strip_null(text.into_owned()), enc.name().to_string());
            }

            // Keep a lossy version as a candidate.
            let lossy = strip_null(text.into_owned());
            let lossy_errors = lossy.matches(HC::UNICODE_REPLACEMENT_CHARACTER).count();

            let mut detector = chardetng::EncodingDetector::new();
            detector.feed(data, true);
            let detected = detector.guess(None, true);
            let (detected_text, _) = detected.decode_with_bom_removal(data);
            let detected_text = strip_null(detected_text.into_owned());
            let detected_errors = detected_text
                .matches(HC::UNICODE_REPLACEMENT_CHARACTER)
                .count();

            if detected_errors == 0 || detected_errors <= lossy_errors {
                return (detected_text, detected.name().to_string());
            }

            return (lossy, enc.name().to_string());
        }
    }

    // No usable encoding: let chardetng have a go, then fall back to 1252.
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(data, true);
    let detected = detector.guess(None, true);
    let (text, _) = detected.decode_with_bom_removal(data);
    let text = strip_null(text.into_owned());

    let (fallback, _) = encoding_rs::WINDOWS_1252.decode_with_bom_removal(data);
    let fallback = strip_null(fallback.into_owned());

    let detected_errors = text.matches(HC::UNICODE_REPLACEMENT_CHARACTER).count();
    let fallback_errors = fallback.matches(HC::UNICODE_REPLACEMENT_CHARACTER).count();

    if detected_errors <= fallback_errors {
        (text, detected.name().to_string())
    } else {
        (fallback, encoding_rs::WINDOWS_1252.name().to_string())
    }
}

fn strip_null(text: String) -> String {
    text.replace(NULL_CHARACTER, "")
}

/// `remove_new_lines`
pub fn remove_new_lines(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("")
}

/// `strip_io_input_line`
pub fn strip_io_input_line(t: &str) -> String {
    let t = t.trim_start_matches(HC::UNICODE_BYTE_ORDER_MARK);

    t.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_sort_key_orders_numbers() {
        let mut texts = vec![
            "Page 19.jpg".to_string(),
            "Page 2.jpg".to_string(),
            "Page 200.jpg".to_string(),
            "Page 21.jpg".to_string(),
        ];

        human_text_sort_strings(&mut texts);

        assert_eq!(
            texts,
            vec![
                "Page 2.jpg".to_string(),
                "Page 19.jpg".to_string(),
                "Page 21.jpg".to_string(),
                "Page 200.jpg".to_string(),
            ]
        );
    }

    #[test]
    fn natural_sort_puts_numbers_before_text() {
        let mut texts = vec!["b".to_string(), "10".to_string(), "a".to_string()];
        human_text_sort_strings(&mut texts);
        assert_eq!(
            texts,
            vec!["10".to_string(), "a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn note_cleaning_collapses_gaps() {
        assert_eq!(
            clean_note_text("  line one  \n\n\n\n line two \n"),
            "line one\n\nline two"
        );
    }

    #[test]
    fn eliding() {
        assert_eq!(elide_text("abcdefghij", 5, false), "abcd\u{2026}");
        assert_eq!(elide_text("abc", 5, false), "abc");
    }

    #[test]
    fn hex_filtering() {
        assert_eq!(hex_filter("AB-CD ef_gh zz"), "abcdef");
    }

    #[test]
    fn json_detection() {
        assert!(looks_like_json_str(r#"{"a": 1}"#));
        assert!(!looks_like_json_str("{oops"));
    }

    #[test]
    fn newline_deserialisation() {
        assert_eq!(
            deserialise_newlined_texts("\u{feff}one\r\n\r\n  two  \n"),
            vec!["one".to_string(), "two".to_string()]
        );
    }

    #[test]
    fn summary_single_line() {
        assert_eq!(
            convert_many_strings_to_nice_insertable_human_summary_single_line(
                &["one"],
                "things",
                true
            ),
            "\"one\""
        );

        assert_eq!(
            convert_many_strings_to_nice_insertable_human_summary_single_line::<&str>(
                &[],
                "things",
                true
            ),
            "0(?) things"
        );
    }

    #[test]
    fn decoding_falls_back() {
        let (text, encoding) = non_failing_unicode_decode(b"caf\xe9", Some("utf-8"), false);
        assert!(text.contains("caf"));
        assert!(!encoding.is_empty());
    }
}
