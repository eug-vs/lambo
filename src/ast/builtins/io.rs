use std::{io::stdin, rc::Rc};

use petgraph::graph::NodeIndex;

use crate::ast::{
    builtins::ConstructorTag, ASTError, ASTResult, Edge, Node, Primitive, VariableKind, AST,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IOTag {
    ReadLine,
    Print,
    Flatmap,
}

impl IOTag {
    pub fn argument_names(&self) -> Vec<&'static str> {
        match self {
            IOTag::ReadLine => vec![],
            IOTag::Print => vec!["bytes"],
            IOTag::Flatmap => vec!["transform", "io"],
        }
    }

    pub fn run(&self, ast: &mut AST, id: NodeIndex) -> ASTResult<NodeIndex> {
        match self {
            IOTag::ReadLine => {
                let mut line = String::new();
                stdin().read_line(&mut line).unwrap();

                Ok(ast
                    .graph
                    .add_node(Node::Primitive(Primitive::Bytes(line.into()))))
            }
            IOTag::Print => {
                let binders = ConstructorTag::get_binders(ast, id);
                let (bytes, is_bytes_dangling) = ast.evaluate_closure_parameter(binders[0])?;

                let value = match ast.graph.node_weight(bytes).unwrap() {
                    Node::Primitive(Primitive::Bytes(bytes)) => bytes,
                    _ => return Err(ASTError::Custom(bytes, "Expected Bytes")),
                };

                print!(
                    "{}",
                    str::from_utf8(&value)
                        .map_err(|_| ASTError::Custom(bytes, "Bytes is not a valid utf8 string"))?
                );
                if is_bytes_dangling {
                    ast.graph.remove_node(bytes);
                }

                Ok(ast
                    .graph
                    .add_node(Node::Variable(VariableKind::Free(Rc::new(
                        "#io_print finished".to_string(),
                    )))))
            }
            IOTag::Flatmap => {
                return Err(ASTError::Custom(id, "#io_flatmap is not an effectful IO"))
            }
        }
    }

    pub fn flatmap(ast: &mut AST, id: NodeIndex) -> ASTResult<NodeIndex> {
        let binders = ConstructorTag::get_binders(ast, id);

        let [trasform_binder, io_binder] = binders
            .try_into()
            .map_err(|_| ASTError::Custom(id, "Incorrect argument count"))?;

        let (io, is_io_dangling) = ast.evaluate_closure_parameter(io_binder)?;

        let io_result = match ast.graph.node_weight(io).unwrap() {
            &Node::Data {
                tag: ConstructorTag::IO(io_tag),
            } => io_tag.run(ast, io)?,
            _ => return Err(ASTError::Custom(id, "Expected IO")),
        };

        if is_io_dangling {
            ast.graph.remove_node(io);
        }

        let (transform, _) = ast.evaluate_closure_parameter(trasform_binder)?;

        let result = ast.graph.add_node(Node::Application);
        ast.graph.add_edge(result, transform, Edge::Function);
        ast.graph.add_edge(result, io_result, Edge::Parameter);

        ast.migrate_node(id, result);
        ast.graph.remove_node(id);

        ast.evaluate(result)
    }
}
