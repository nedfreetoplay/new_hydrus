//! Tree transformations: operator conversion, De Morgan, distribution and
//! flattening.

use super::LogicExpr;

fn make_node(op: &str, children: Vec<LogicExpr>) -> LogicExpr {
    LogicExpr::node(op, children)
}

fn negate(expression: LogicExpr) -> LogicExpr {
    make_node("not", vec![convert_to_and_or_not(&expression)])
}

fn map_children(node: &super::Node, function: fn(&LogicExpr) -> LogicExpr) -> LogicExpr {
    LogicExpr::Node(super::Node {
        op: node.op.clone(),
        children: node.children.iter().map(function).collect(),
    })
}

/// `convert_to_and_or_not( node )`
pub fn convert_to_and_or_not(expression: &LogicExpr) -> LogicExpr {
    let LogicExpr::Node(node) = expression else {
        return expression.clone();
    };

    match node.op.as_str() {
        "implies" => make_node(
            "or",
            vec![
                negate(node.children[0].clone()),
                convert_to_and_or_not(&node.children[1]),
            ],
        ),
        "xor" => make_node(
            "or",
            vec![
                make_node(
                    "and",
                    vec![
                        convert_to_and_or_not(&node.children[0]),
                        negate(node.children[1].clone()),
                    ],
                ),
                make_node(
                    "and",
                    vec![
                        negate(node.children[0].clone()),
                        convert_to_and_or_not(&node.children[1]),
                    ],
                ),
            ],
        ),
        "nor" => negate(make_node("or", node.children.clone())),
        "nand" => negate(make_node("and", node.children.clone())),
        "iff" => make_node(
            "or",
            vec![
                convert_to_and_or_not(&make_node("and", node.children.clone())),
                make_node(
                    "and",
                    vec![
                        negate(node.children[0].clone()),
                        negate(node.children[1].clone()),
                    ],
                ),
            ],
        ),
        _ => map_children(node, convert_to_and_or_not),
    }
}

/// `move_not_inwards( node )`
pub fn move_not_inwards(expression: &LogicExpr) -> LogicExpr {
    let LogicExpr::Node(node) = expression else {
        return expression.clone();
    };

    if node.op == "not" {
        if let LogicExpr::Node(child) = &node.children[0] {
            match child.op.as_str() {
                "not" => return move_not_inwards(&child.children[0]),
                "and" => {
                    return make_node(
                        "or",
                        vec![
                            move_not_inwards(&make_node("not", vec![child.children[0].clone()])),
                            move_not_inwards(&make_node("not", vec![child.children[1].clone()])),
                        ],
                    );
                }
                "or" => {
                    return make_node(
                        "and",
                        vec![
                            move_not_inwards(&make_node("not", vec![child.children[0].clone()])),
                            move_not_inwards(&make_node("not", vec![child.children[1].clone()])),
                        ],
                    );
                }
                _ => {}
            }
        }
    }

    map_children(node, move_not_inwards)
}

/// `distribute_and_over_or( node )`
pub fn distribute_and_over_or(expression: &LogicExpr) -> LogicExpr {
    let LogicExpr::Node(node) = expression else {
        return expression.clone();
    };

    let children: Vec<LogicExpr> = node.children.iter().map(distribute_and_over_or).collect();

    if node.op != "or" {
        return LogicExpr::Node(super::Node {
            op: node.op.clone(),
            children,
        });
    }

    if let LogicExpr::Node(first) = &children[0] {
        if first.op == "and" {
            let a = first.children[0].clone();
            let b = first.children[1].clone();
            let c = children[1].clone();

            return make_node(
                "and",
                vec![
                    distribute_and_over_or(&make_node("or", vec![a, c.clone()])),
                    distribute_and_over_or(&make_node("or", vec![b, c])),
                ],
            );
        }
    }

    if let LogicExpr::Node(second) = &children[1] {
        if second.op == "and" {
            let a = second.children[0].clone();
            let b = second.children[1].clone();
            let c = children[0].clone();

            return make_node(
                "and",
                vec![
                    distribute_and_over_or(&make_node("or", vec![a, c.clone()])),
                    distribute_and_over_or(&make_node("or", vec![b, c])),
                ],
            );
        }
    }

    LogicExpr::Node(super::Node {
        op: node.op.clone(),
        children,
    })
}

/// `flatten_tree( node )`
pub fn flatten_tree(expression: &LogicExpr) -> LogicExpr {
    let LogicExpr::Node(node) = expression else {
        return expression.clone();
    };

    let children: Vec<LogicExpr> = node.children.iter().map(flatten_tree).collect();

    if node.op == "and" || node.op == "or" {
        let mut flattened = Vec::new();

        for child in children {
            match &child {
                LogicExpr::Node(child_node) if child_node.op == node.op => {
                    flattened.extend(child_node.children.iter().cloned());
                }
                _ => flattened.push(child),
            }
        }

        return LogicExpr::Node(super::Node {
            op: node.op.clone(),
            children: flattened,
        });
    }

    LogicExpr::Node(super::Node {
        op: node.op.clone(),
        children,
    })
}
