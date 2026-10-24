
use super::*;

#[test]
fn conversion_chain_matches_python_example() {
    let converter = StringConverter::new(vec![
        Conversion::RemoveTextFromBeginning(1),
        Conversion::RemoveTextFromEnd(1),
        Conversion::ClipTextFromBeginning(7),
        Conversion::ClipTextFromEnd(6),
        Conversion::PrependText("abc".to_string()),
        Conversion::AppendText("x z".to_string()),
        Conversion::Encode(EncodingType::UrlPercent),
        Conversion::Decode(EncodingType::UrlPercent),
        Conversion::Reverse,
        Conversion::RegexSub(r"\d".to_string(), "d".to_string()),
    ]);

    assert_eq!(converter.convert("0123456789"), "z xddddddcba");
}

#[test]
fn encodings_round_trip() {
    for encoding in [
        EncodingType::UrlPercent,
        EncodingType::UnicodeEscape,
        EncodingType::HtmlEntities,
        EncodingType::HexUtf8,
        EncodingType::Base64Utf8,
        EncodingType::Base64UrlUtf8,
    ] {
        let text = "01234 Δ&56789";
        let encoded = encode(text, encoding);
        assert_eq!(decode(&encoded, encoding), text, "{encoding:?}");
    }

    assert_eq!(
        encode("01234Δ56789", EncodingType::UnicodeEscape),
        "01234\\u039456789"
    );
    assert_eq!(
        encode("01234 56789", EncodingType::UrlPercent),
        "01234%2056789"
    );
    assert_eq!(
        encode("hello world", EncodingType::HexUtf8),
        "68656c6c6f20776f726c64"
    );
    assert_eq!(
        encode("hello world", EncodingType::Base64Utf8),
        "aGVsbG8gd29ybGQ="
    );
    assert_eq!(
        encode("hello world", EncodingType::Base64UrlUtf8),
        "aGVsbG8gd29ybGQ"
    );
}

#[test]
fn dates_and_integers_convert() {
    let decode_date = StringConverter::new(vec![Conversion::DateDecode(
        "%Y-%m-%d %H:%M:%S".to_string(),
    )]);
    assert_eq!(decode_date.convert("1970-01-02 00:00:00"), "86400");

    let encode_date = StringConverter::new(vec![Conversion::DateEncode(
        "%Y-%m-%d %H:%M:%S".to_string(),
    )]);
    assert_eq!(encode_date.convert("86400"), "1970-01-02 00:00:00");

    let addition = StringConverter::new(vec![Conversion::IntegerAddition(5)]);
    assert_eq!(addition.convert("4"), "9");
}

#[test]
fn joiner_supports_tuple_sizes() {
    let strings: Vec<String> = ["ab", "cd", "ef", "gh", "ij"]
        .iter()
        .map(|value| value.to_string())
        .collect();

    assert_eq!(
        StringJoiner::new(",", None).join(&strings),
        vec!["ab,cd,ef,gh,ij"]
    );
    assert_eq!(
        StringJoiner::new("--", Some(2)).join(&strings),
        vec!["ab--cd", "ef--gh"]
    );
}

#[test]
fn processor_splits_filters_and_converts() {
    let processor = StringProcessor::new(vec![
        ProcessingStep::Splitter(StringSplitter::with_max_splits(",", 2)),
        ProcessingStep::Match(StringMatch::flexible(
            crate::parsing::string_match::FlexibleMatch::Numeric,
        )),
        ProcessingStep::Converter(StringConverter::new(vec![Conversion::AppendText(
            "abc".to_string(),
        )])),
    ]);

    assert_eq!(
        processor.process_strings(["1,a,2,3".to_string(), "test".to_string(), "123".to_string()]),
        vec!["1abc", "123abc"]
    );
}
