
use super::*;

#[test]
fn counts_merge_like_python() {
    assert_eq!(merge_counts(100, 100, 100, 100), (100, 200));
    assert_eq!(merge_counts(1, 1, 4, 5), (4, 6));
    assert_eq!(merge_counts(1, 2, 5, 7), (5, 9));
}

#[test]
fn zoom_string_matches_python_formatting() {
    assert_eq!(convert_zoom_to_percentage(1.0), "100%");
    assert_eq!(convert_zoom_to_percentage(0.125), "12.50%");
}

#[test]
fn credentials_match_python_behaviour() {
    let credentials = Credentials::new("localhost", 45871, Some(vec![0xab, 0xcd]));

    assert_eq!(credentials.get_address(), ("127.0.0.1", 45871));
    assert_eq!(credentials.get_connection_string(), "abcd@127.0.0.1:45871");
    assert_eq!(credentials.get_ported_address(), "127.0.0.1:45871");
    assert!(credentials.has_access_key());
}

#[test]
fn ascii_helpers_match_python() {
    assert!(ord_is_sensible_ascii(b' '));
    assert!(ord_is_sensible_ascii(b'~'));
    assert!(!ord_is_sensible_ascii(31));
    assert!(ord_is_alpha(b'Z'));
    assert!(ord_is_number(b'7'));
}
