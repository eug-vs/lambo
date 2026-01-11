use std::{fmt::Debug, rc::Rc};

use petgraph::{graph::NodeIndex, stable_graph::EdgeReference};

use crate::ast::{builtins::arithmetic::ArithmeticTag, ASTResult, Edge, Node, VariableKind, AST};

pub mod arithmetic;
// pub mod helpers;
// pub mod io;

#[derive(Debug, Clone, Copy)]
pub enum ConstructorTag {
    // IO(IOTag),
    Arithmetic(ArithmeticTag),
    // HelperFunction(HelperFunctionTag),
    // CustomTag { uid: usize, arity: usize },
}

impl ConstructorTag {
    pub fn from_str(str: &str) -> Option<Self> {
        match str {
            // "#match" => Some(Self::HelperFunction(HelperFunctionTag::Match)),
            // "#constructor" => Some(Self::HelperFunction(HelperFunctionTag::CreateConstructor)),
            //
            // "#io_getchar" => Some(Self::IO(IOTag::GetChar)),
            // "#io_putchar" => Some(Self::IO(IOTag::PutChar)),
            // "#io_throw" => Some(Self::IO(IOTag::Throw)),
            // "#io_flatmap" => Some(Self::IO(IOTag::Flatmap)),
            "=num" => Some(Self::Arithmetic(ArithmeticTag::Eq)),
            "+" => Some(Self::Arithmetic(ArithmeticTag::Add)),
            "-" => Some(Self::Arithmetic(ArithmeticTag::Sub)),
            "*" => Some(Self::Arithmetic(ArithmeticTag::Mul)),
            "/" => Some(Self::Arithmetic(ArithmeticTag::Div)),
            "^" => Some(Self::Arithmetic(ArithmeticTag::Pow)),

            _ => None,
        }
    }
    pub fn argument_names(&self) -> Vec<&str> {
        match self {
            // Self::IO(tag) => tag.argument_names(),
            Self::Arithmetic(tag) => tag.argument_names(),
            // Self::HelperFunction(tag) => tag.argument_names(),
            // Self::CustomTag { arity, .. } => {
            //     vec!["param"; *arity]
            // }
        }
    }

    pub fn get_arguments(ast: &mut AST, id: NodeIndex) -> Vec<EdgeReference<'_, Edge>> {
        let mut arguments = ast
            .graph
            .edges_directed(id, petgraph::Direction::Outgoing)
            .collect::<Vec<_>>();

        arguments.sort_by_key(|e| match *e.weight() {
            Edge::ConstructorArgument(argument_index) => argument_index,
            _ => panic!(),
        });

        arguments
    }

    pub fn arity(&self) -> usize {
        self.argument_names().len()
    }

    pub fn evaluate(&self, ast: &mut AST, id: NodeIndex) -> ASTResult<NodeIndex> {
        match self {
            Self::Arithmetic(tag) => tag.evaluate(ast, id),
            // Self::HelperFunction(tag) => tag.evaluate(ast, arguments),
            // Self::IO(IOTag::Flatmap) => IOTag::flatmap(ast, arguments),
            // Self::CustomTag { .. } | Self::IO { .. } => Ok(()),
        }
    }
}

impl AST {
    pub fn add_constructor(&mut self, tag: ConstructorTag) -> NodeIndex {
        let data = self.graph.add_node(Node::Data { tag });
        let mut child = data;
        for (argument_index, argument_name) in tag.argument_names().iter().enumerate().rev() {
            let argument_name = Rc::new(argument_name.to_string());
            let lambda = self.graph.add_node(Node::Lambda {
                argument_name: argument_name.clone(),
            });
            self.graph.add_edge(lambda, child, Edge::Body);
            child = lambda;

            let var = self.graph.add_node(Node::Variable(VariableKind::Bound));
            self.graph.add_edge(var, lambda, Edge::Binder);
            self.graph
                .add_edge(data, var, Edge::ConstructorArgument(argument_index));
        }
        child
    }
}
