use std::{iter::Peekable, panic, rc::Rc};

use crate::{
    evaluator::{builtins::ConstructorTag, Graph, Node, Primitive, VariableKind},
    parser::lexer::Token,
};

type BindingPower = usize;

fn binding_power(token: &Token) -> (BindingPower, BindingPower) {
    match token {
        Token::Pipe => (10, 11), // Very small binding power for pipe | operator
        _ => (100, 101),         // Everything else is left-associative
    }
}

/// Parse Token iterator into an Expression
pub fn parse_expr<I: Iterator<Item = Token>>(
    graph: &mut Graph,
    tokens: &mut Peekable<I>,
    min_binding_power: BindingPower,
    mut ctx: Vec<String>,
) -> usize {
    let mut lhs = match tokens.next().unwrap() {
        Token::Symbol(name) => {
            let name = Rc::new(name);
            let kind = match ctx.iter().rev().position(|n| *n == *name) {
                Some(depth) => VariableKind::Bound {
                    depth: depth + 1, // Just to avoid 0, purely sugar
                },
                None => VariableKind::Free,
            };
            if matches!(kind, VariableKind::Free) {
                if let Some(tag) = ConstructorTag::from_str(&name) {
                    graph.add_constructor(tag)
                } else if let Ok(number) = name.parse::<usize>() {
                    graph.add_node(Node::Primitive(Primitive::Number(number)))
                } else {
                    graph.add_node(Node::Var { name, kind })
                }
            } else {
                graph.add_node(Node::Var { name, kind })
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
            let body = parse_expr(graph, tokens, 0, ctx.clone());
            graph.add_node(Node::Lambda {
                argument: Rc::new(variable_name),
                body,
            })
        }
        Token::OpenParen => {
            let result = parse_expr(graph, tokens, 0, ctx.clone());
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
            let value = parse_expr(graph, tokens, 0, ctx.clone());
            match tokens.next() {
                Some(Token::In) => {}
                token => panic!("Expected In, got: {:?}", token),
            };

            ctx.push(variable_name.clone());
            let body = parse_expr(graph, tokens, 0, ctx.clone());
            let lambda = graph.add_node(Node::Lambda {
                argument: Rc::new(variable_name),
                body,
            });
            graph.add_node(Node::Call {
                function: lambda,
                parameter: value,
            })
        }
        token => panic!("Invalid syntax: unexpected token {:?}", token),
    };
    loop {
        let next_token = match tokens.peek().unwrap() {
            Token::Eof | Token::CloseParen | Token::In => break,
            token => token,
        };
        let (l_bp, r_bp) = binding_power(next_token);
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

        let rhs = parse_expr(graph, tokens, r_bp, ctx.clone());

        lhs = graph.add_node(match next_token {
            // Pipe swaps rhs and lhs: (value | f1 | f2) parses into (f2 (f1 value))
            Token::Pipe => Node::Call {
                function: rhs,
                parameter: lhs,
            },
            _ => Node::Call {
                function: lhs,
                parameter: rhs,
            },
        });
    }
    lhs
}
