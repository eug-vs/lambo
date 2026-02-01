use crate::ast::{AST, ASTError, ASTResult, Edge, Node, VariableKind, builtins::ConstructorTag};
use petgraph::graph::NodeIndex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HelperFunctionTag {
    /// Meta-constructor to create constructors at runtime
    CreateConstructor,
    Match,
}

impl HelperFunctionTag {
    pub fn argument_names(&self) -> Vec<&'static str> {
        match self {
            Self::CreateConstructor => vec!["arity"],
            Self::Match => vec!["constructor", "transform", "fallback", "value"],
        }
    }

    pub fn evaluate(&self, ast: &mut AST, id: NodeIndex) -> ASTResult<NodeIndex> {
        let binders = ConstructorTag::get_binders(ast, id);
        match self {
            Self::CreateConstructor => {
                let [arity_binder] = binders.try_into().map_err(|_| {
                    ASTError::Custom(id, "Incorrect argument count for CreateConstructor")
                })?;
                let arity = ast
                    .extract_primitive_from_environment(arity_binder)
                    .and_then(|p| p.extract_number())?;

                let tag = ConstructorTag::CustomTag {
                    uid: ast.next_uid(),
                    arity,
                };

                let constructor = ast.graph.add_node(Node::Data { tag });
                ast.migrate_node(id, constructor);
                ast.graph.remove_node(id);
                Ok(constructor)
            }
            Self::Match => {
                let [constructor, transform, fallback, value_binder] = binders
                    .as_slice()
                    .try_into()
                    .map_err(|_| ASTError::Custom(id, "Incorrect argument count for Match"))?;

                // We are strict only in constructor and value
                let (constructor, _is_constructor_dangling) =
                    ast.evaluate_closure_parameter(constructor)?;
                let (value, is_value_dangling) = ast.evaluate_closure_parameter(value_binder)?;

                let value_tag_uid = match ast.graph.node_weight(value).unwrap() {
                    Node::Data {
                        tag: ConstructorTag::CustomTag { uid, .. },
                    } => uid,
                    _ => return Err(ASTError::Custom(value, "Not a data constructor")),
                };

                let (constructor_tag_uid, constructor_id) = {
                    let mut current = constructor;
                    loop {
                        let edge = match ast.graph.node_weight(current).unwrap() {
                            Node::Closure { .. } | Node::Lambda { .. } => Edge::Body,
                            Node::Application => Edge::Function,
                            Node::Data { .. } => break,
                            _ => unreachable!(),
                        };
                        current = ast.follow_edge(current, edge)?;
                    }
                    match ast.graph.node_weight(current).unwrap() {
                        Node::Data {
                            tag: ConstructorTag::CustomTag { uid, .. },
                            ..
                        } => (uid, current),
                        _ => unreachable!(), // Not really
                    }
                };

                if constructor_tag_uid == value_tag_uid {
                    let mut chain = ConstructorTag::get_binders(ast, value)
                        .iter()
                        .map(|&constructor_binder| {
                            let var = ast.graph.add_node(Node::Variable(VariableKind::Bound));
                            ast.graph.add_edge(var, constructor_binder, Edge::Binder(0));
                            let application = ast.graph.add_node(Node::Application);
                            ast.graph.add_edge(application, var, Edge::Parameter);
                            application
                        })
                        .rev()
                        .collect::<Vec<_>>();

                    if is_value_dangling {
                        ast.graph.remove_node(value);
                    }

                    let transform_var = ast.graph.add_node(Node::Variable(VariableKind::Bound));
                    ast.graph
                        .add_edge(transform_var, transform, Edge::Binder(0));
                    chain.push(transform_var);

                    for window in chain.windows(2) {
                        ast.graph.add_edge(window[0], window[1], Edge::Function);
                    }

                    let head = *chain.first().unwrap();
                    ast.migrate_node(id, head);
                    ast.graph.remove_node(id);
                    ast.evaluate(head)
                } else {
                    // Call fallback function with value again
                    // Such API allows easier chaining of #match expressions
                    let fallback_var = ast.graph.add_node(Node::Variable(VariableKind::Bound));
                    ast.graph.add_edge(fallback_var, fallback, Edge::Binder(0));

                    let value = if is_value_dangling {
                        value
                    } else {
                        let value_var = ast.graph.add_node(Node::Variable(VariableKind::Bound));
                        ast.graph.add_edge(value_var, value_binder, Edge::Binder(0));
                        value_var
                    };

                    let application = ast.graph.add_node(Node::Application);
                    ast.graph
                        .add_edge(application, fallback_var, Edge::Function);
                    ast.graph.add_edge(application, value, Edge::Parameter);

                    ast.migrate_node(id, application);
                    ast.graph.remove_node(id);
                    ast.evaluate(application)
                }
            }
        }
    }
}
