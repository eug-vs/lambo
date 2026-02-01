use crate::ast::{builtins::ConstructorTag, ASTError, ASTResult, Node, Number, Primitive, AST};
use petgraph::graph::NodeIndex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BytesOpTag {
    New,
    Get,
    Set,
    Length,
    Push,
    Pop,
}

impl BytesOpTag {
    pub fn argument_names(&self) -> Vec<&'static str> {
        match self {
            Self::New => vec!["size"],
            Self::Get => vec!["index", "bytes"],
            Self::Set => vec!["index", "value", "bytes"],
            Self::Length => vec!["bytes"],
            Self::Push => vec!["value", "bytes"],
            Self::Pop => vec!["bytes"],
        }
    }

    pub fn evaluate(&self, ast: &mut AST, id: NodeIndex) -> ASTResult<NodeIndex> {
        let binders = ConstructorTag::get_binders(ast, id);
        match self {
            Self::New => {
                let size = ast
                    .extract_primitive_from_environment(binders[0])
                    .map(|p| p.extract_number())
                    .flatten()?;

                let bytes = vec![0; size];
                let node = ast.graph.add_node(Node::Primitive(Primitive::Bytes(bytes)));

                ast.migrate_node(id, node);
                ast.graph.remove_node(id);

                Ok(node)
            }
            Self::Get => {
                let [index_binder, byte_array_binder] = binders
                    .try_into()
                    .map_err(|_| ASTError::Custom(id, "Incorrect argument count"))?;

                let index = ast
                    .extract_primitive_from_environment(index_binder)
                    .map(|p| p.extract_number())
                    .flatten()?;

                let (byte_array_id, is_dangling) =
                    ast.evaluate_closure_parameter(byte_array_binder)?;

                let value = match ast.graph.node_weight(byte_array_id).unwrap() {
                    Node::Primitive(Primitive::Bytes(byte_array)) => byte_array[index],
                    _ => return Err(ASTError::Custom(byte_array_id, "Expected Bytes")),
                };

                if is_dangling {
                    ast.graph.remove_node(byte_array_id);
                }

                let node = ast
                    .graph
                    .add_node(Node::Primitive(Primitive::Number(value as Number)));

                ast.migrate_node(id, node);
                ast.graph.remove_node(id);

                Ok(node)
            }
            Self::Length => {
                let (byte_array_id, is_dangling) = ast.evaluate_closure_parameter(binders[0])?;

                let value = match ast.graph.node_weight(byte_array_id).unwrap() {
                    Node::Primitive(Primitive::Bytes(byte_array)) => byte_array.len(),
                    _ => return Err(ASTError::Custom(byte_array_id, "Expected Bytes")),
                };

                if is_dangling {
                    ast.graph.remove_node(byte_array_id);
                }

                let node = ast
                    .graph
                    .add_node(Node::Primitive(Primitive::Number(value as Number)));

                ast.migrate_node(id, node);
                ast.graph.remove_node(id);

                Ok(node)
            }
            Self::Push => {
                let [value_binder, byte_array_binder] = binders
                    .try_into()
                    .map_err(|_| ASTError::Custom(id, "Incorrect argument count"))?;

                let value = ast
                    .extract_primitive_from_environment(value_binder)
                    .map(|p| p.extract_number())
                    .flatten()?;

                let mut bytes = match ast.extract_primitive_from_environment(byte_array_binder)? {
                    Primitive::Bytes(bytes) => bytes,
                    _ => return Err(ASTError::Custom(id, "Expected Bytes")),
                };

                bytes.push(
                    value
                        .try_into()
                        .map_err(|_| ASTError::Custom(id, "Value larger than byte"))?,
                );

                let node = ast.graph.add_node(Node::Primitive(Primitive::Bytes(bytes)));

                ast.migrate_node(id, node);
                ast.graph.remove_node(id);

                Ok(node)
            }
            _ => todo!(),
        }
    }
}
