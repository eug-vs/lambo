use std::{iter::Peekable, panic, rc::Rc};

use petgraph::graph::NodeIndex;

use crate::{
    ast::{builtins::ConstructorTag, Edge, Node, Primitive, VariableKind, AST},
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
    ast: &mut AST,
    tokens: &mut Peekable<I>,
    min_binding_power: BindingPower,
    mut lambda_ctx: Vec<String>,
) -> NodeIndex {
    let mut lhs = match tokens.next().unwrap() {
        Token::Symbol(name) => {
            let name = Rc::new(name);
            let kind = match lambda_ctx.iter().rev().position(|n| *n == *name) {
                Some(depth) => VariableKind::Bound {
                    depth: depth + 1, // Just to avoid 0, purely sugar
                },
                None => VariableKind::Free,
            };
            if matches!(kind, VariableKind::Free) {
                if let Some(tag) = ConstructorTag::from_str(&name) {
                    ast.add_constructor(tag)
                } else if let Ok(number) = name.parse::<usize>() {
                    ast.graph
                        .add_node(Node::Primitive(Primitive::Number(number)))
                } else {
                    ast.graph.add_node(Node::Variable { name, kind })
                }
            } else {
                ast.graph.add_node(Node::Variable { name, kind })
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
            lambda_ctx.push(variable_name.clone());
            let body = parse_expr(ast, tokens, 0, lambda_ctx.clone());

            let lambda_node = ast.graph.add_node(Node::Lambda {
                argument_name: Rc::new(variable_name),
            });
            ast.graph.add_edge(lambda_node, body, Edge::Body);
            lambda_node
        }
        Token::OpenParen => {
            let result = parse_expr(ast, tokens, 0, lambda_ctx.clone());
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
            let value = parse_expr(ast, tokens, 0, lambda_ctx.clone());
            match tokens.next() {
                Some(Token::In) => {}
                token => panic!("Expected In, got: {:?}", token),
            };

            lambda_ctx.push(variable_name.clone());
            let body = parse_expr(ast, tokens, 0, lambda_ctx.clone());

            let closure_node = ast.graph.add_node(Node::Closure {
                argument_name: Rc::new(variable_name),
            });
            let body_edge = ast.graph.add_edge(closure_node, body, Edge::Body);
            let parameter_edge = ast.graph.add_edge(closure_node, value, Edge::Parameter);

            closure_node
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

        let rhs = parse_expr(ast, tokens, r_bp, lambda_ctx.clone());
        let app_node = ast.graph.add_node(Node::Application);

        match next_token {
            // Pipe swaps rhs and lhs: (value | f1 | f2) parses into (f2 (f1 value))
            Token::Pipe => {
                ast.graph.add_edge(app_node, rhs, Edge::Function);
                ast.graph.add_edge(app_node, lhs, Edge::Parameter);
            }
            _ => {
                ast.graph.add_edge(app_node, rhs, Edge::Parameter);
                ast.graph.add_edge(app_node, lhs, Edge::Function);
            }
        };

        lhs = app_node
    }
    lhs
}
