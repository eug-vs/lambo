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
    mut binder_ctx: Vec<NodeIndex>,
) -> NodeIndex {
    let mut lhs = match tokens.next().unwrap() {
        Token::Symbol(name) => {
            let name = Rc::new(name);
            match binder_ctx.iter().rfind(|index| {
                if let Some(Node::Lambda { argument_name } | Node::Closure { argument_name }) =
                    ast.graph.node_weight(**index)
                {
                    return *argument_name == name;
                }
                panic!("lambda_ctx elements can only point to lambda/closure nodes")
            }) {
                Some(binder_id) => {
                    let node = ast.graph.add_node(Node::Variable(VariableKind::Bound));
                    ast.graph.add_edge(node, *binder_id, Edge::Binder(0));
                    node
                }
                None => {
                    if let Ok(tag) = ConstructorTag::try_from(name.as_str()) {
                        ast.graph.add_node(Node::Data { tag })
                    } else if let Ok(number) = name.parse::<usize>() {
                        ast.graph
                            .add_node(Node::Primitive(Primitive::Number(number)))
                    } else {
                        ast.graph.add_node(Node::Variable(VariableKind::Free(name)))
                    }
                }
            }
        }
        Token::Lambda => {
            // Support nested syntax: \x y z.x y z
            let mut lambdas_chain = vec![];
            while let Some(Token::Symbol(_)) = tokens.peek() {
                let Some(Token::Symbol(variable_name)) = tokens.next() else {
                    unreachable!()
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
                let lambda_node = ast.graph.add_node(Node::Lambda {
                    argument_name: Rc::new(variable_name),
                });
                binder_ctx.push(lambda_node);
                lambdas_chain.push(lambda_node);
            }
            match tokens.next() {
                Some(Token::Dot) => {}
                token => panic!("Expected DOT, got: {:?}", token),
            }
            let head = *lambdas_chain
                .first()
                .expect("At least one lambda node must have been created!");

            let body = parse_expr(ast, tokens, 0, binder_ctx.clone());
            lambdas_chain.push(body);

            for window in lambdas_chain.windows(2) {
                ast.graph.add_edge(window[0], window[1], Edge::Body);
            }

            head
        }
        Token::OpenParen => {
            let result = parse_expr(ast, tokens, 0, binder_ctx.clone());
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
            let value = parse_expr(ast, tokens, 0, binder_ctx.clone());
            match tokens.next() {
                Some(Token::In) => {}
                token => panic!("Expected In, got: {:?}", token),
            };
            let closure_node = ast.graph.add_node(Node::Closure {
                argument_name: Rc::new(variable_name),
            });

            binder_ctx.push(closure_node);
            let body = parse_expr(ast, tokens, 0, binder_ctx.clone());

            ast.graph.add_edge(closure_node, body, Edge::Body);
            ast.graph.add_edge(closure_node, value, Edge::Parameter);

            closure_node
        }
        Token::Quoted(quoted) => ast
            .graph
            .add_node(Node::Primitive(Primitive::Bytes(quoted.into()))),
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

        let rhs = parse_expr(ast, tokens, r_bp, binder_ctx.clone());
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
