//! Rust port of `hydrus_python/hydrus/external/LogicExpressionQueryParser.py`.
//!
//! The parser accepts boolean tag expressions such as
//! `(a || b) and not c`, turns them into a tree and returns the equivalent
//! list of clauses. Every clause is a set of terms; a leading `-` means the
//! term is negated.

mod parse;
mod simplify;
mod tokenizer;
mod transform;

pub use parse::parse;
pub use simplify::convert_to_list_and_simplify;
pub use tokenizer::{Token, next_token};
pub use transform::{
    convert_to_and_or_not, distribute_and_over_or, flatten_tree, move_not_inwards,
};

use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// A logical expression: either a tag or an operator node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicExpr {
    Tag(String),
    Node(Node),
}

impl LogicExpr {
    pub fn tag(tag: impl Into<String>) -> Self {
        Self::Tag(tag.into())
    }

    pub fn node(op: impl Into<String>, children: Vec<Self>) -> Self {
        Self::Node(Node {
            op: op.into(),
            children,
        })
    }

    pub fn op(&self) -> Option<&str> {
        match self {
            Self::Node(node) => Some(node.op.as_str()),
            Self::Tag(_) => None,
        }
    }

    pub fn children(&self) -> Option<&[Self]> {
        match self {
            Self::Node(node) => Some(&node.children),
            Self::Tag(_) => None,
        }
    }
}

/// A node in a logical expression tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub op: String,
    pub children: Vec<LogicExpr>,
}

impl std::fmt::Display for LogicExpr {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tag(tag) => formatter.write_str(tag),
            Self::Node(node) => {
                if node.op == "not" {
                    write!(formatter, "not ({})", node.children[0])
                } else {
                    let children: Vec<String> = node
                        .children
                        .iter()
                        .map(|child| format!("({child})"))
                        .collect();

                    formatter.write_str(&children.join(&format!(" {} ", node.op)))
                }
            }
        }
    }
}

pub(crate) fn parse_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::Parse, message)
}

/// `parse_logic_expression_query( input_str )`
pub fn parse_logic_expression_query(input: &str) -> HydrusResult<Vec<std::collections::BTreeSet<String>>> {
    let tree = parse(input)?;
    let tree = convert_to_and_or_not(&tree);
    let tree = move_not_inwards(&tree);
    let tree = distribute_and_over_or(&tree);
    let tree = flatten_tree(&tree);

    Ok(convert_to_list_and_simplify(&tree))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clauses(input: &str) -> Vec<std::collections::BTreeSet<String>> {
        parse_logic_expression_query(input).unwrap()
    }

    #[test]
    fn simple_terms_and_implicit_precedence() {
        assert_eq!(clauses("a or b").len(), 1);
        assert_eq!(clauses("a and b").len(), 2);
        assert_eq!(clauses("a and b or c").len(), 2);
    }

    #[test]
    fn parentheses_and_negation_work() {
        let result = clauses("(a || b) and not c");

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|clause| clause.contains("-c")));
    }

    #[test]
    fn all_operators_are_supported() {
        for expression in [
            "a implies b",
            "a xor b",
            "a nor b",
            "a nand b",
            "a xnor b",
            "a iff b",
        ] {
            assert!(!clauses(expression).is_empty(), "{expression}");
        }
    }

    #[test]
    fn escapes_allow_operator_text_in_tags() {
        let result = clauses(r"a \or b");

        assert_eq!(result, vec![std::collections::BTreeSet::from(["a or b".to_string()])]);
    }

    #[test]
    fn tautologies_are_filtered() {
        let result = clauses("a or not a");

        assert_eq!(result, vec![std::collections::BTreeSet::from(["a".to_string(), "-a".to_string()])]);
    }

    #[test]
    fn syntax_errors_are_reported() {
        for expression in ["", "a and", "(a", "a )"] {
            assert!(parse_logic_expression_query(expression).is_err(), "{expression}");
        }
    }
}
