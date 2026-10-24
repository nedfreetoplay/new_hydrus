//! Tokenizer for [`super`], ported from `next_token`.

use hydrus_core::hydrus_exceptions::HydrusResult;

use super::parse_error;

/// One lexical token. `xnor` is normalized to [`Token::Iff`] like Python.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    End,
    Not,
    And,
    Or,
    Implies,
    Xor,
    Nor,
    Nand,
    Iff,
    LParen,
    RParen,
    Tag(String),
}

impl Token {
    pub fn op_name(&self) -> Option<&'static str> {
        match self {
            Self::Not => Some("not"),
            Self::And => Some("and"),
            Self::Or => Some("or"),
            Self::Implies => Some("implies"),
            Self::Xor => Some("xor"),
            Self::Nor => Some("nor"),
            Self::Nand => Some("nand"),
            Self::Iff => Some("iff"),
            _ => None,
        }
    }

    pub fn precedence(&self) -> Option<u8> {
        match self {
            Self::Not => Some(10),
            Self::And => Some(9),
            Self::Or => Some(8),
            Self::Nor | Self::Nand => Some(7),
            Self::Xor => Some(6),
            Self::Implies => Some(5),
            Self::Iff => Some(4),
            _ => None,
        }
    }
}

fn keyword(rest: &str) -> Option<(Token, usize)> {
    for (name, token) in [
        ("not", Token::Not),
        ("and", Token::And),
        ("or", Token::Or),
        ("implies", Token::Implies),
        ("xor", Token::Xor),
        ("nor", Token::Nor),
        ("nand", Token::Nand),
        ("xnor", Token::Iff),
        ("iff", Token::Iff),
    ] {
        if let Some(after) = rest.strip_prefix(name) {
            if after.starts_with('(')
                || after.chars().next().is_some_and(char::is_whitespace)
            {
                return Some((token, name.len()));
            }
        }
    }

    None
}

fn check_tag_end(rest: &str) -> bool {
    if ["&&", "||", "=>", "<=>", ")", "("]
        .iter()
        .any(|operator| rest.starts_with(operator))
    {
        return true;
    }

    let mut chars = rest.chars();

    if let Some(first) = chars.next() {
        if first.is_whitespace() {
            let after = chars.as_str();
            return ["and", "or", "implies", "xor", "nor", "nand", "xnor", "iff"]
                .iter()
                .any(|operator| after.starts_with(operator));
        }
    }

    false
}

/// `next_token( src )`, returning the token and the unconsumed input.
pub fn next_token(src: &str) -> HydrusResult<(Token, &str)> {
    let mut rest = src.trim_start();

    if rest.is_empty() {
        return Ok((Token::End, rest));
    }

    let mut escape = false;

    if rest.starts_with('\\') && rest.len() > 1 {
        escape = true;
        rest = &rest[1..];
    }

    if !escape {
        for (prefix, token, length) in [
            ("!", Token::Not, 1),
            ("-", Token::Not, 1),
            ("&&", Token::And, 2),
            ("||", Token::Or, 2),
            ("=>", Token::Implies, 2),
            ("<=>", Token::Iff, 3),
            ("(", Token::LParen, 1),
            (")", Token::RParen, 1),
        ] {
            if rest.starts_with(prefix) {
                return Ok((token, &rest[length..]));
            }
        }

        if let Some((token, length)) = keyword(rest) {
            return Ok((token, &rest[length..]));
        }
    }

    let mut tag = String::new();

    if escape {
        let character = rest.chars().next().expect("non-empty escaped token");
        tag.push(character);
        rest = &rest[character.len_utf8()..];
    }

    while !rest.is_empty() && !check_tag_end(rest) {
        if rest.starts_with('\\') && rest.len() > 1 {
            let character = rest[1..]
                .chars()
                .next()
                .expect("non-empty escaped tag character");
            tag.push(character);
            rest = &rest[1 + character.len_utf8()..];
        } else {
            let character = rest.chars().next().expect("non-empty tag");
            tag.push(character);
            rest = &rest[character.len_utf8()..];
        }
    }

    let tag = tag.trim().to_string();

    if tag.is_empty() {
        return Err(parse_error("Syntax error: empty search term"));
    }

    Ok((Token::Tag(tag), rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_keep_tag_text_and_advance_input() {
        let (token, rest) = next_token("hello and world").unwrap();

        assert_eq!(token, Token::Tag("hello".to_string()));
        assert_eq!(rest, " and world");

        let (token, _) = next_token(rest).unwrap();
        assert_eq!(token, Token::And);
    }

    #[test]
    fn escapes_protect_operator_text() {
        let (token, rest) = next_token(r"a \or b").unwrap();

        assert_eq!(token, Token::Tag("a or b".to_string()));
        assert_eq!(rest, "");
    }

    #[test]
    fn xnor_is_normalized_to_iff() {
        assert_eq!(next_token("xnor a").unwrap().0, Token::Iff);
    }
}
