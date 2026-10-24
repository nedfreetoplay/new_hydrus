//! CNF clause conversion and tautology filtering.

use std::collections::BTreeSet;

use super::{LogicExpr, Node};

fn term(expression: &LogicExpr) -> String {
    match expression {
        LogicExpr::Tag(tag) => tag.clone(),
        LogicExpr::Node(node) if node.op == "not" => {
            format!("-{}", term(&node.children[0]))
        }
        _ => String::new(),
    }
}

fn term_from_negation(expression: &LogicExpr) -> String {
    match expression {
        LogicExpr::Node(node) if node.op == "not" => {
            format!("-{}", term(&node.children[0]))
        }
        _ => term(expression),
    }
}

fn clause_from_children(children: &[LogicExpr]) -> BTreeSet<String> {
    children.iter().map(term_from_negation).collect()
}

/// `convert_to_list_and_simplify( node )`
pub fn convert_to_list_and_simplify(expression: &LogicExpr) -> Vec<BTreeSet<String>> {
    let mut clauses: Vec<BTreeSet<String>> = Vec::new();

    match expression {
        LogicExpr::Tag(tag) => {
            clauses.push(BTreeSet::from([tag.clone()]));
        }
        LogicExpr::Node(Node { op, children }) => match op.as_str() {
            "and" => {
                for child in children {
                    match child {
                        LogicExpr::Tag(tag) => {
                            clauses.push(BTreeSet::from([tag.clone()]));
                        }
                        LogicExpr::Node(node) if node.op == "not" => {
                            clauses.push(BTreeSet::from([term(child)]));
                        }
                        LogicExpr::Node(node) => {
                            clauses.push(clause_from_children(&node.children));
                        }
                    }
                }
            }
            "or" => clauses.push(clause_from_children(children)),
            "not" => clauses.push(BTreeSet::from([format!("-{}", term(&children[0]))])),
            _ => {}
        },
    }

    let mut filtered: Vec<Option<BTreeSet<String>>> = Vec::with_capacity(clauses.len());
    let mut last_always_true_clause: Option<BTreeSet<String>> = None;

    for clause in clauses {
        let mut always_true = false;

        for item in &clause {
            let negated = if let Some(stripped) = item.strip_prefix('-') {
                stripped.to_string()
            } else {
                format!("-{item}")
            };

            if clause.contains(&negated) {
                always_true = true;
                last_always_true_clause = Some(clause.clone());
                break;
            }
        }

        if !always_true {
            filtered.push(Some(clause));
        }
    }

    for i in 0..filtered.len() {
        for j in 0..filtered.len() {
            if i != j && filtered[i].is_some() && filtered[i] == filtered[j] {
                filtered[i] = None;
            }
        }
    }

    let mut result: Vec<BTreeSet<String>> = filtered.into_iter().flatten().collect();

    if result.is_empty() {
        if let Some(clause) = last_always_true_clause {
            result.push(clause);
        }
    }

    result
}
