//! Shunting-yard parser and RPN tree builder.

use hydrus_core::hydrus_exceptions::HydrusResult;

use super::tokenizer::{Token, next_token};
use super::{LogicExpr, parse_error};

fn is_tag_or_rparen(token: &Token) -> bool {
    matches!(token, Token::Tag(_) | Token::RParen)
}

/// `parse( src )`
pub fn parse(src: &str) -> HydrusResult<LogicExpr> {
    let mut rest = src.to_lowercase();
    let mut previous = Token::End;
    let mut output: Vec<Token> = Vec::new();
    let mut operators: Vec<Token> = Vec::new();

    loop {
        let (token, remaining) = next_token(&rest)?;
        rest = remaining.to_string();
        let previous_token = token.clone();

        match token {
            Token::End => break,
            Token::Tag(_) => output.push(token),
            Token::LParen => operators.push(token),
            Token::RParen => {
                while operators.last().is_some_and(|top| *top != Token::LParen) {
                    output.push(operators.pop().expect("checked above"));
                }

                if operators.pop() != Some(Token::LParen) {
                    return Err(parse_error("Syntax error: mismatched parentheses"));
                }
            }
            _ => {
                if token == Token::Not && is_tag_or_rparen(&previous) {
                    return Err(parse_error("Syntax error: invalid negation"));
                }

                while let Some(top) = operators.last() {
                    if *top == Token::LParen {
                        break;
                    }

                    let top_precedence = top.precedence().ok_or_else(|| {
                        parse_error(format!("Syntax error: '{top:?}' is not an operator"))
                    })?;
                    let precedence = token.precedence().ok_or_else(|| {
                        parse_error(format!("Syntax error: '{token:?}' is not an operator"))
                    })?;

                    if top_precedence > precedence
                        || (top_precedence == precedence && *top != Token::Not)
                    {
                        output.push(operators.pop().expect("checked above"));
                    } else {
                        break;
                    }
                }

                operators.push(token);
            }
        }

        previous = previous_token;
    }

    while let Some(token) = operators.pop() {
        if matches!(token, Token::LParen | Token::RParen) {
            return Err(parse_error("Syntax error: mismatched parentheses"));
        }

        output.push(token);
    }

    if output.is_empty() {
        return Err(parse_error("Empty input!"));
    }

    build_tree(output)
}

fn build_tree(rpn: Vec<Token>) -> HydrusResult<LogicExpr> {
    let mut stack: Vec<LogicExpr> = Vec::new();

    for token in rpn {
        match token {
            Token::Tag(tag) => stack.push(LogicExpr::Tag(tag)),
            Token::Not => {
                let child = stack
                    .pop()
                    .ok_or_else(|| parse_error("Syntax error: wrong number of arguments"))?;

                stack.push(LogicExpr::node("not", vec![child]));
            }
            Token::End | Token::LParen | Token::RParen => {}
            operator => {
                if stack.len() < 2 {
                    return Err(parse_error("Syntax error: wrong number of arguments"));
                }

                let right = stack.pop().expect("checked above");
                let left = stack.pop().expect("checked above");
                let op = operator.op_name().expect("operator token");

                stack.push(LogicExpr::node(op, vec![left, right]));
            }
        }
    }

    if stack.len() != 1 {
        return Err(parse_error("Parser error: unused values left in stack"));
    }

    Ok(stack.pop().expect("checked above"))
}
