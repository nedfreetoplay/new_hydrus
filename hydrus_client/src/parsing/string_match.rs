//! String matchers from `hydrus/client/ClientStrings.py`.

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MatchType {
    #[default]
    All,
    Fixed(String),
    Regex(String),
    Flexible(FlexibleMatch),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexibleMatch {
    Alpha,
    AlphaNumeric,
    Numeric,
    Hex,
    Base64,
    Base64UrlEncoded,
    Base64Url,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StringMatch {
    pub match_type: MatchType,
    pub min_chars: Option<usize>,
    pub max_chars: Option<usize>,
}

impl StringMatch {
    pub fn all() -> Self {
        Self::default()
    }

    pub fn fixed(value: impl Into<String>) -> Self {
        Self {
            match_type: MatchType::Fixed(value.into()),
            ..Default::default()
        }
    }

    pub fn flexible(value: FlexibleMatch) -> Self {
        Self {
            match_type: MatchType::Flexible(value),
            ..Default::default()
        }
    }

    pub fn matches(&self, text: &str) -> bool {
        let length = text.chars().count();

        if let Some(min_chars) = self.min_chars {
            if length < min_chars {
                return false;
            }
        }

        if let Some(max_chars) = self.max_chars {
            if length > max_chars {
                return false;
            }
        }

        match &self.match_type {
            MatchType::All => true,
            MatchType::Fixed(value) => text == value,
            MatchType::Regex(pattern) => {
                Regex::new(pattern).is_ok_and(|regex| regex.is_match(text))
            }
            MatchType::Flexible(kind) => matches_flexible(text, *kind),
        }
    }

    pub fn to_string(&self) -> String {
        let body = match &self.match_type {
            MatchType::All => "anything".to_string(),
            MatchType::Fixed(value) => format!("exactly \"{value}\""),
            MatchType::Regex(value) => format!("regex \"{value}\""),
            MatchType::Flexible(kind) => format!("{kind:?}"),
        };

        match (self.min_chars, self.max_chars) {
            (None, None) => body,
            (Some(min), None) => format!("{body}, at least {min} chars"),
            (None, Some(max)) => format!("{body}, at most {max} chars"),
            (Some(min), Some(max)) => format!("{body}, between {min} and {max} chars"),
        }
    }
}

fn matches_flexible(text: &str, kind: FlexibleMatch) -> bool {
    if text.is_empty() {
        return false;
    }

    match kind {
        FlexibleMatch::Alpha => text.chars().all(char::is_alphabetic),
        FlexibleMatch::AlphaNumeric => text.chars().all(char::is_alphanumeric),
        FlexibleMatch::Numeric => text.chars().all(|c| c.is_ascii_digit()),
        FlexibleMatch::Hex => text.chars().all(|c| c.is_ascii_hexdigit()),
        FlexibleMatch::Base64 => {
            let body = text.trim_end_matches('=');

            body.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/'))
        }
        FlexibleMatch::Base64UrlEncoded => text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '%' | '+' | '/' | '=')),
        FlexibleMatch::Base64Url => {
            let body = text.trim_end_matches('=');

            body.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_constraints_are_applied() {
        let matcher = StringMatch {
            min_chars: Some(4),
            max_chars: Some(6),
            ..Default::default()
        };

        assert!(!matcher.matches("abc"));
        assert!(matcher.matches("abcd"));
        assert!(!matcher.matches("abcdefg"));
    }

    #[test]
    fn fixed_and_regex_match_whole_or_part() {
        assert!(StringMatch::fixed("123").matches("123"));
        assert!(!StringMatch::fixed("123").matches("abc123"));

        let regex = StringMatch {
            match_type: MatchType::Regex(r"\d".to_string()),
            ..Default::default()
        };

        assert!(regex.matches("abc123"));
        assert!(!regex.matches("abc"));
    }

    #[test]
    fn flexible_kinds_follow_python_examples() {
        assert!(StringMatch::flexible(FlexibleMatch::Alpha).matches("abc"));
        assert!(!StringMatch::flexible(FlexibleMatch::Alpha).matches("abc123"));
        assert!(
            StringMatch::flexible(FlexibleMatch::AlphaNumeric).matches("abc123"),
            "alphanumeric accepts letters and digits"
        );
        assert!(!StringMatch::flexible(FlexibleMatch::AlphaNumeric).matches("abc123@"));
        assert!(StringMatch::flexible(FlexibleMatch::Hex).matches("abc123"));
        assert!(!StringMatch::flexible(FlexibleMatch::Hex).matches("abc123z"));
    }

    #[test]
    fn base64_variants_accept_their_alphabets() {
        assert!(StringMatch::flexible(FlexibleMatch::Base64).matches("abc123+/="));
        assert!(!StringMatch::flexible(FlexibleMatch::Base64).matches("abc123+]"));
        assert!(StringMatch::flexible(FlexibleMatch::Base64Url).matches("abc123_="));
        assert!(!StringMatch::flexible(FlexibleMatch::Base64Url).matches("abc123+]"));
    }
}
