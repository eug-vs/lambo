use std::{iter::Peekable, panic};

use crate::{Expr, Variable, VariableKind, parser::lexer::Token};

type BindingPower = f32;

fn binding_power(token: &Token) -> (BindingPower, BindingPower) {
    match token {
        _ => (1.0, 1.1), // Everything is left-associative
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
                Some(depth) => VariableKind::Bound(depth + 1), // Just to avoid 0, purely sugar
                None => VariableKind::Free,
            };
            Expr::Var(Variable {
                name: name.clone(),
                kind,
            })
        }
        Token::Lambda => {
            let variable_name = match tokens.next().unwrap() {
                Token::Symbol(name) => name,
                token => panic!("Expected variable name, got: {:?}", token),
            };
            ctx.push(variable_name.clone());
            match tokens.next().unwrap() {
                Token::Dot => {}
                token => panic!("Expected DOT, got: {:?}", token),
            }
            let body = parse_expr(tokens, min_binding_power, ctx.clone());
            Expr::Lambda(variable_name.clone(), Box::new(body))
        }
        Token::OpenParen => {
            let result = parse_expr(tokens, 0.0, ctx.clone());
            match tokens.next().unwrap() {
                Token::CloseParen => {}
                token => panic!("Expected CloseParen, got: {:?}", token),
            }
            result
        }
        token => panic!("Invalid syntax: unexpected token {:?}", token),
    };
    loop {
        let next_token = match tokens.peek().unwrap() {
            Token::Eof | Token::CloseParen => break,
            token => token,
        };
        let (l_bp, r_bp) = binding_power(&next_token);
        if l_bp < min_binding_power {
            break;
        }
        let rhs = parse_expr(tokens, r_bp, ctx.clone());
        lhs = Expr::Call(Box::new(lhs), Box::new(rhs));
    }
    lhs
}
