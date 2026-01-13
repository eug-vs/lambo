use std::fmt::Debug;

use petgraph::{graph::NodeIndex, visit::EdgeRef};

use crate::ast::{
    builtins::{arithmetic::ArithmeticTag, helpers::HelperFunctionTag},
    ASTError, ASTResult, Edge, Node, Primitive, AST,
};

pub mod arithmetic;
pub mod helpers;
// pub mod io;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstructorTag {
    // IO(IOTag),
    Arithmetic(ArithmeticTag),
    HelperFunction(HelperFunctionTag),
    CustomTag { uid: usize, arity: usize },
}

const TAGS: &[(&str, ConstructorTag)] = &[
    (
        "#constructor",
        ConstructorTag::HelperFunction(HelperFunctionTag::CreateConstructor),
    ),
    (
        "#match",
        ConstructorTag::HelperFunction(HelperFunctionTag::Match),
    ),
    ("=num", ConstructorTag::Arithmetic(ArithmeticTag::Eq)),
    ("+", ConstructorTag::Arithmetic(ArithmeticTag::Add)),
    ("-", ConstructorTag::Arithmetic(ArithmeticTag::Sub)),
    ("*", ConstructorTag::Arithmetic(ArithmeticTag::Mul)),
    ("/", ConstructorTag::Arithmetic(ArithmeticTag::Div)),
    ("^", ConstructorTag::Arithmetic(ArithmeticTag::Pow)),
];

impl TryFrom<&str> for ConstructorTag {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        TAGS.iter()
            .find(|(k, _)| *k == value)
            .map(|(_, v)| *v)
            .ok_or(())
    }
}

impl Into<String> for ConstructorTag {
    fn into(self) -> String {
        match self {
            ConstructorTag::CustomTag { uid, .. } => format!("CustomTag{uid}"),
            _ => TAGS
                .iter()
                .find(|(_, t)| *t == self)
                .map(|(k, _)| k.to_string())
                .unwrap(),
        }
    }
}

impl ConstructorTag {
    pub fn argument_names(&self) -> Vec<&str> {
        match self {
            // Self::IO(tag) => tag.argument_names(),
            Self::Arithmetic(tag) => tag.argument_names(),
            Self::HelperFunction(tag) => tag.argument_names(),
            Self::CustomTag { arity, .. } => {
                vec!["param"; *arity]
            }
        }
    }

    pub fn get_binders(ast: &mut AST, id: NodeIndex) -> Vec<NodeIndex> {
        let mut edges = ast
            .graph
            .edges_directed(id, petgraph::Direction::Outgoing)
            .collect::<Vec<_>>();

        edges.sort_by_key(|e| match *e.weight() {
            Edge::Binder(argument_index) => argument_index,
            _ => panic!(),
        });

        edges.into_iter().map(|e| e.target()).collect()
    }

    pub fn arity(&self) -> usize {
        self.argument_names().len()
    }

    pub fn evaluate(&self, ast: &mut AST, id: NodeIndex) -> ASTResult<NodeIndex> {
        match self {
            Self::Arithmetic(tag) => tag.evaluate(ast, id),
            Self::HelperFunction(tag) => tag.evaluate(ast, id),
            Self::CustomTag { .. } => Ok(id)
            // Self::IO(IOTag::Flatmap) => IOTag::flatmap(ast, arguments),
            // Self::CustomTag { .. } | Self::IO { .. } => Ok(()),
        }
    }
}

impl AST {
    pub fn extract_primitive_from_environment(
        &mut self,
        closure_id: NodeIndex,
    ) -> ASTResult<Primitive> {
        let (parameter, is_dangling) = self.evaluate_closure_parameter(closure_id)?;
        let primitive = if is_dangling {
            self.graph.remove_node(parameter)
        } else {
            self.graph.node_weight(parameter).cloned()
        };

        match primitive {
            Some(Node::Primitive(primitive)) => Ok(primitive),
            _ => Err(ASTError::Custom(closure_id, "Not a primitive")),
        }
    }
}
