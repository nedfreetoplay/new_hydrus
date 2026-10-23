//! Rust port of `hydrus/core/files/HydrusFilesPhysicalStorage.py`.
//!
//! Files are stored below a two-hex-digit prefix directory. The helpers here
//! are the pure path arithmetic; moving bytes around lives in the client's
//! storage layer.

use std::collections::HashSet;

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// `DEFAULT_PREFIX_LENGTH`
pub const DEFAULT_PREFIX_LENGTH: usize = 2;

/// `get_prefix( hash, prefix_type, prefix_length )`
pub fn get_prefix(hash: &[u8], prefix_type: &str, prefix_length: usize) -> String {
    let mut prefix = String::with_capacity(prefix_type.len() + prefix_length);

    prefix.push_str(prefix_type);

    for byte in hash {
        if prefix.len() >= prefix_type.len() + prefix_length {
            break;
        }

        prefix.push(char::from_digit((byte >> 4) as u32, 16).unwrap());
        prefix.push(char::from_digit((byte & 0x0F) as u32, 16).unwrap());
    }

    prefix.truncate(prefix_type.len() + prefix_length);

    prefix
}

/// `iterate_prefixes( prefix_type, prefix_length )`
///
/// Python yields every combination lazily. The port materialises the list; the
/// default prefix length is two, so this is at most 256 entries.
pub fn iterate_prefixes(prefix_type: &str, prefix_length: usize) -> Vec<String> {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    if prefix_length == 0 {
        return vec![prefix_type.to_string()];
    }

    let mut prefixes = vec![prefix_type.to_string()];

    for _ in 0..prefix_length {
        let mut next = Vec::with_capacity(prefixes.len() * HEX_CHARS.len());

        for prefix in &prefixes {
            for digit in HEX_CHARS {
                let mut generated = prefix.clone();
                generated.push(char::from(*digit));
                next.push(generated);
            }
        }

        prefixes = next;
    }

    prefixes
}

/// `get_missing_prefixes( prefix_type, prefixes_found, prefix_length )`
pub fn get_missing_prefixes<S>(
    prefix_type: &str,
    prefixes_found: &HashSet<S>,
    prefix_length: usize,
) -> Vec<String>
where
    S: AsRef<str> + std::hash::Hash + Eq,
{
    let expected = iterate_prefixes(prefix_type, prefix_length);

    let found: HashSet<&str> = prefixes_found
        .iter()
        .map(|prefix| prefix.as_ref())
        .collect();

    let mut missing: Vec<String> = expected
        .into_iter()
        .filter(|prefix| !found.contains(prefix.as_str()))
        .collect();

    missing.sort();

    missing
}

/// `check_full_prefix_coverage( prefix_type, prefixes_found, prefix_length )`
pub fn check_full_prefix_coverage<S>(
    prefix_type: &str,
    prefixes_found: &HashSet<S>,
    prefix_length: usize,
) -> HydrusResult<()>
where
    S: AsRef<str> + std::hash::Hash + Eq,
{
    let missing_prefixes = get_missing_prefixes(prefix_type, prefixes_found, prefix_length);

    if missing_prefixes.is_empty() {
        return Ok(());
    }

    let list_of_problems = missing_prefixes.join(", ");

    Err(HydrusException::new(
        ErrorKind::DataMissing,
        format!("Missing storage spaces! They are: {list_of_problems}"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_uses_the_first_hex_digits() {
        let hash = [0x12, 0xAB, 0xCD];

        assert_eq!(get_prefix(&hash, "f", 2), "f12");
        assert_eq!(get_prefix(&hash, "t", 4), "t12ab");
        assert_eq!(get_prefix(&hash, "", 1), "1");
    }

    #[test]
    fn prefix_length_zero_is_just_the_type() {
        assert_eq!(get_prefix(&[0x12], "f", 0), "f");
        assert_eq!(iterate_prefixes("f", 0), vec!["f"]);
    }

    #[test]
    fn all_two_character_prefixes_are_generated_in_order() {
        let prefixes = iterate_prefixes("f", 2);

        assert_eq!(prefixes.len(), 256);
        assert_eq!(prefixes.first().map(String::as_str), Some("f00"));
        assert_eq!(prefixes.last().map(String::as_str), Some("fff"));
        assert_eq!(prefixes[17], "f11");
    }

    #[test]
    fn missing_prefixes_are_sorted_and_coverage_is_checked() {
        let mut found = iterate_prefixes("f", 2).into_iter().collect::<HashSet<_>>();

        assert!(check_full_prefix_coverage("f", &found, 2).is_ok());

        let removed = "f0a".to_string();
        found.remove(&removed);

        let missing = get_missing_prefixes("f", &found, 2);

        assert_eq!(missing, vec!["f0a"]);

        let error = check_full_prefix_coverage("f", &found, 2).unwrap_err();

        assert!(error.is_a(ErrorKind::DataMissing));
        assert!(error.to_string().contains("f0a"));
    }
}
