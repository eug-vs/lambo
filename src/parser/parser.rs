use std::{iter::Peekable, panic};

use crate::{Expr, VariableKind, parser::lexer::Token};

type BindingPower = usize;

fn binding_power(token: &Token) -> (BindingPower, BindingPower) {
    match token {
        Token::Pipe => (10, 11), // Very small binding power for pipe | operator
        _ => (100, 101),         // Everything else is left-associative
    }
}

/// Parse Token iterator into an Expression
pub fn parse_expr<I: Iterator<Item = Token>>(
    tokens: &mut Peekable<I>,
    min_binding_power: BindingPower,
    mut ctx: Vec<String>,
) -> Expr {
    let mut lhs = match tokens.next().unwrap() {
        Token::Symbol(name) => {
            let kind = match ctx.iter().rev().position(|n| *n == name) {
                Some(depth) => VariableKind::Bound {
                    depth: depth + 1, // Just to avoid 0, purely sugar
                },
                None => VariableKind::Free,
            };
            Expr::Var {
                name: name.clone(),
                kind,
            }
        }
        Token::Lambda => {
            let variable_name = match tokens.next() {
                Some(Token::Symbol(name)) => name,
                token => panic!("Expected variable name, got: {:?}", token),
            };
            match tokens.peek() {
                Some(Token::Colon) => {
                    tokens.next(); // Consume :
                    match tokens.next() {
                        Some(Token::Symbol(_type_name)) => {} // TODO: do something with type
                        token => panic!("Expected type, got: {:?}", token),
                    };
                }
                _ => {} // TODO: Default to any type
            };
            match tokens.next() {
                Some(Token::Dot) => {}
                token => panic!("Expected DOT, got: {:?}", token),
            }
            ctx.push(variable_name.clone());
            let body = parse_expr(tokens, 0, ctx.clone());
            Expr::Lambda {
                argument: variable_name,
                body: Box::new(body),
            }
        }
        Token::OpenParen => {
            let result = parse_expr(tokens, 0, ctx.clone());
            match tokens.next() {
                Some(Token::CloseParen) => {}
                token => panic!("Expected CloseParen, got: {:?}", token),
            }
            result
        }
        Token::With => {
            let variable_name = match tokens.next() {
                Some(Token::Symbol(name)) => name,
                token => panic!("Expected variable name, got: {:?}", token),
            };
            let value = parse_expr(tokens, 0, ctx.clone());
            match tokens.next() {
                Some(Token::In) => {}
                token => panic!("Expected In, got: {:?}", token),
            };
            let expr = parse_expr(tokens, 0, ctx.clone());
            expr.provide_variable(variable_name.as_str(), value)
        }
        token => panic!("Invalid syntax: unexpected token {:?}", token),
    };
    loop {
        let next_token = match tokens.peek().unwrap() {
            Token::Eof | Token::CloseParen | Token::In => break,
            token => token,
        };
        let (l_bp, r_bp) = binding_power(&next_token);
        if l_bp < min_binding_power {
            break;
        }

        // Clone to not lose the referenced object
        let next_token = next_token.clone();

        // Some tokens we have to consume
        match next_token {
            Token::Pipe | Token::Colon => {
                tokens.next().unwrap();
            }
            _ => {}
        };

        let rhs = parse_expr(tokens, r_bp, ctx.clone());

        lhs = match next_token {
            // Pipe swaps rhs and lhs: (value | f1 | f2) parses into (f2 (f1 value))
            Token::Pipe => Expr::Call {
                function: Box::new(rhs),
                parameter: Box::new(lhs),
            },
            _ => Expr::Call {
                function: Box::new(lhs),
                parameter: Box::new(rhs),
            },
        };
    }
    lhs
}
