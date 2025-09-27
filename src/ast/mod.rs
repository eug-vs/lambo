use std::{collections::HashSet, fmt::Display, rc::Rc};

pub mod builtins;
mod debug;

use petgraph::{
    dot::Dot,
    graph::{EdgeIndex, NodeIndex},
    prelude::StableGraph,
    stable_graph::EdgeReference,
    visit::EdgeRef,
    Direction,
};

use crate::ast::builtins::ConstructorTag;

#[derive(Debug, Clone)]
pub enum VariableKind {
    Free,
    Bound { depth: usize },
}

pub type Number = usize;

#[derive(Debug, Clone)]
pub enum Primitive {
    Number(Number),
}

#[derive(Debug, Clone)]
pub enum DebugNode {
    Annotation { text: String },
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Edge {
    Body,
    Parameter,
    Function,
    Debug,
    ConstructorArgument(usize),
}

#[derive(Debug, Clone)]
pub enum Node {
    Lambda {
        argument_name: Rc<String>,
    },
    Application,
    Variable {
        name: Rc<String>,
        kind: VariableKind,
    },
    Primitive(Primitive),
    Closure {
        argument_name: Rc<String>,
    },
    Data {
        tag: ConstructorTag,
    },
    Debug(DebugNode),
}

pub struct AST {
    pub graph: StableGraph<Node, Edge>,
    pub root: NodeIndex,

    debug_frames: Vec<String>,
}

#[derive(Debug)]
pub enum ASTError {
    EdgeNotFound(NodeIndex, Edge),
    ParentError(NodeIndex),
    InvalidClosureChain,
    Custom(NodeIndex, &'static str),
}

type ASTResult<T> = Result<T, ASTError>;

pub struct LambdaDepthTraverser {
    stack: Vec<(NodeIndex, usize)>,
}

impl LambdaDepthTraverser {
    fn new(root: NodeIndex) -> Self {
        Self {
            stack: vec![(root, 0)],
        }
    }
    fn next(&mut self, graph: &StableGraph<Node, Edge>) -> Option<(NodeIndex, usize)> {
        let (id, depth) = self.stack.pop()?;

        for edge in graph.edges_directed(id, Direction::Outgoing) {
            let depth_increment = match edge.weight() {
                Edge::Body => 1,
                _ => 0,
            };
            self.stack.push((edge.target(), depth + depth_increment));
        }
        Some((id, depth))
    }
}

impl AST {
    pub fn new() -> Self {
        Self {
            root: NodeIndex::default(),
            graph: StableGraph::new(),
            debug_frames: Vec::new(),
        }
    }
    fn get_edge_ref<'a>(
        &'a self,
        expr: NodeIndex,
        edge: Edge,
    ) -> ASTResult<EdgeReference<'a, Edge>> {
        self.graph
            .edges(expr)
            .find(|e| *e.weight() == edge)
            .ok_or(ASTError::EdgeNotFound(expr, edge))
    }
    fn follow_edge(&self, expr: NodeIndex, edge: Edge) -> ASTResult<NodeIndex> {
        self.get_edge_ref(expr, edge).map(|e| e.target())
    }
    fn redirect_edge(&mut self, edge_id: EdgeIndex, node: NodeIndex) {
        let (source, _) = self.graph.edge_endpoints(edge_id).unwrap();
        let edge = self.graph.remove_edge(edge_id).unwrap();
        self.graph.add_edge(source, node, edge);
    }
    fn migrate_node(&mut self, from: NodeIndex, to: NodeIndex) {
        for edge in self
            .graph
            .edges_directed(from, Direction::Incoming)
            .map(|e| e.id())
            .collect::<Vec<_>>()
        {
            self.redirect_edge(edge, to)
        }

        if self.root == from {
            self.root = to;
        }
    }
    pub fn fmt_expr(&self, expr: NodeIndex, tab_index: usize) -> ASTResult<String> {
        let indent = "  ".repeat(tab_index);
        match &self.graph[expr] {
            Node::Variable { name, .. } => Ok(name.to_string()),
            Node::Lambda { argument_name } => Ok(format!(
                "λ{}.{}",
                argument_name,
                self.fmt_expr(self.follow_edge(expr, Edge::Body)?, tab_index)?
            )),
            Node::Application => Ok(format!(
                "({} {})",
                self.fmt_expr(self.follow_edge(expr, Edge::Function)?, tab_index)?,
                self.fmt_expr(self.follow_edge(expr, Edge::Parameter)?, tab_index)?
            )),
            Node::Primitive(Primitive::Number(number)) => Ok(format!("{}", number)),
            Node::Closure { argument_name, .. } => Ok(format!(
                "{indent}let {} \n{indent}{} in\n{indent}{}",
                argument_name,
                self.fmt_expr(self.follow_edge(expr, Edge::Parameter)?, tab_index + 1)?,
                self.fmt_expr(self.follow_edge(expr, Edge::Body)?, tab_index)?,
            )),
            Node::Debug(_) => Ok(String::new()),
            Node::Data { tag } => {
                let mut edges = self
                    .graph
                    .edges_directed(expr, Direction::Outgoing)
                    .collect::<Vec<_>>();
                edges.sort_by_key(|e| match *e.weight() {
                    Edge::ConstructorArgument(argument_index) => argument_index,
                    _ => panic!(),
                });
                Ok(format!(
                    "({:?} {})",
                    tag,
                    edges
                        .into_iter()
                        .map(|e| self.fmt_expr(e.target(), tab_index))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(" ")
                ))
            }
        }
    }
    fn clone_subtree(&mut self, node_id: NodeIndex) -> NodeIndex {
        let cloned_id = self
            .graph
            .add_node(self.graph.node_weight(node_id).unwrap().clone());

        let edges = self
            .graph
            .edges_directed(node_id, Direction::Outgoing)
            .map(|e| (e.target(), *e.weight()))
            .collect::<Vec<_>>();

        for (target, weight) in edges {
            let cloned_target = self.clone_subtree(target);
            self.graph.add_edge(cloned_id, cloned_target, weight);
        }
        cloned_id
    }

    fn adjust_depth(&mut self, id: NodeIndex, by: isize) {
        let mut traverser = LambdaDepthTraverser::new(id);

        while let Some((index, lambda_depth)) = traverser.next(&self.graph) {
            match self.graph.node_weight_mut(index).unwrap() {
                Node::Variable {
                    kind: VariableKind::Bound { depth },
                    ..
                } if *depth > lambda_depth => {
                    if by > 0 {
                        *depth += by as usize;
                    } else {
                        *depth -= -by as usize;
                    }
                }
                _ => {}
            }
        }
    }

    fn get_closure_chain(&self, closure: NodeIndex) -> (Vec<NodeIndex>, NodeIndex) {
        let mut closure_chain = vec![closure];
        loop {
            let id = *closure_chain.last().unwrap();
            match self.graph.node_weight(id).unwrap() {
                Node::Closure { .. } => {
                    closure_chain.push(self.follow_edge(id, Edge::Body).unwrap())
                }
                _ => {
                    let under_closures = closure_chain.pop().unwrap();
                    return (closure_chain, under_closures);
                }
            }
        }
    }

    /// Lifts environment above the current node and returns the length of lifted closure chain
    fn lift_closure_chain(&mut self, node_id: NodeIndex, edge: Edge) -> ASTResult<()> {
        // println!("Lifting {:?} in {}", edge, self.fmt_expr(node_id)?);
        let (edge_id, edge_target) = self
            .get_edge_ref(node_id, edge)
            .map(|edge_ref| (edge_ref.id(), edge_ref.target()))?;

        let (closure_chain, node_under_closures) = self.get_closure_chain(edge_target);

        if !closure_chain.is_empty() {
            // Closure chain on a function position: LIFT!
            let first_closure = *closure_chain.first().unwrap();

            // Parent now points to a closure chain
            self.migrate_node(node_id, first_closure);

            // Closure chain now points to current node
            self.migrate_node(node_under_closures, node_id);

            // Current edge now points to whatever was under closure chain
            self.redirect_edge(edge_id, node_under_closures);

            // Every child node has gained new binders,
            // except for the node that was already under closures
            self.adjust_depth(node_id, closure_chain.len() as isize);
            self.adjust_depth(node_under_closures, -(closure_chain.len() as isize));
            // ^ this is probably incorrect, we likely need a blacklist to adjust_depth
        }

        self.add_debug_frame();
        Ok(())
    }

    fn get_parent(&self, id: NodeIndex) -> ASTResult<(NodeIndex, Edge)> {
        let mut iter = self
            .graph
            .edges_directed(id, Direction::Incoming)
            .filter_map(|e| {
                if matches!(
                    e.weight(),
                    Edge::Parameter | Edge::Function | Edge::Body | Edge::ConstructorArgument(_)
                ) {
                    Some((e.source(), *e.weight()))
                } else {
                    None
                }
            });

        let result = iter.next().ok_or(ASTError::ParentError(id))?;

        debug_assert!(iter.next().is_none(), "Expected to have only one parent");
        Ok(result)
    }

    fn get_closure(&self, mut id: NodeIndex) -> ASTResult<NodeIndex> {
        loop {
            let (parent_id, edge_from_parent) = self.get_parent(id)?;
            let parent = self.graph.node_weight(parent_id).unwrap();
            match (parent, edge_from_parent) {
                (Node::Closure { .. }, Edge::Body) => return Ok(parent_id),
                (Node::Lambda { .. }, Edge::Body) => return Ok(parent_id),
                _ => id = parent_id,
            };
        }
    }

    fn find_closure_at_depth(&self, mut id: NodeIndex, mut depth: usize) -> ASTResult<NodeIndex> {
        while depth > 0 {
            id = self.get_closure(id)?;
            depth -= 1;
        }
        Ok(id)
    }

    fn debug_node(&self, id: NodeIndex) {
        println!("Node at ID {:?}: {:?}", id, self.graph.node_weight(id));
        println!("Children:");
        for edge in self.graph.edges(id) {
            println!(
                "{:?}: {:?}",
                edge.weight(),
                self.graph.node_weight(edge.target())
            )
        }

        println!("\nParents:");
        for edge in self.graph.edges_directed(id, Direction::Incoming) {
            println!(
                "{:?}: {:?}",
                edge.weight(),
                self.graph.node_weight(edge.target())
            )
        }
    }

    pub fn debug_ast_error(&self, error: ASTError) {
        println!("\n\n{:?}", error);
        let id = match error {
            ASTError::EdgeNotFound(id, edge) => id,
            ASTError::ParentError(id) => id,
            ASTError::Custom(id, _) => id,
            _ => todo!(),
        };
        self.debug_node(id);
    }

    pub fn evaluate(&mut self, node_id: NodeIndex) -> Result<(), ASTError> {
        self.add_debug_frame_with_annotation(node_id, "evaluate");
        match *self.graph.node_weight(node_id).unwrap() {
            Node::Closure { .. } => {
                let body = self.follow_edge(node_id, Edge::Body)?;
                return self.evaluate(body);
            }
            Node::Application => {
                self.evaluate(self.follow_edge(node_id, Edge::Function)?)?;
                self.lift_closure_chain(node_id, Edge::Function)?;

                let (function_edge, function_target) = self
                    .get_edge_ref(node_id, Edge::Function)
                    .map(|e| (e.id(), e.target()))
                    .unwrap();

                if let Node::Lambda { argument_name } =
                    self.graph.node_weight(function_target).unwrap()
                {
                    let argument_name = argument_name.clone();

                    // Current application node becomes a closure
                    *self.graph.node_weight_mut(node_id).unwrap() = Node::Closure { argument_name };

                    // Remove the function edge from the current node
                    self.graph.remove_edge(function_edge);

                    // Add body edge to the closure instead
                    let (body_id, body_target) = self
                        .get_edge_ref(function_target, Edge::Body)
                        .map(|e| (e.id(), e.target()))
                        .unwrap();
                    self.graph.add_edge(node_id, body_target, Edge::Body);

                    // Cleanup lambda node and its edges
                    self.graph.remove_edge(body_id);
                    self.graph.remove_node(function_target);

                    // Parameter edge already exists from the application node

                    return self.evaluate(node_id);
                }
            }
            Node::Variable {
                kind: VariableKind::Bound { depth },
                ..
            } => {
                self.check_variable_integrity(node_id);

                let binding_closure_id = self.find_closure_at_depth(node_id, depth)?;
                self.evaluate(self.follow_edge(binding_closure_id, Edge::Parameter)?)?;
                self.lift_closure_chain(binding_closure_id, Edge::Parameter)?;

                let cloned_node_id =
                    self.clone_subtree(self.follow_edge(binding_closure_id, Edge::Parameter)?);
                self.migrate_node(node_id, cloned_node_id);
                self.graph.remove_node(node_id);
                self.adjust_depth(cloned_node_id, depth as isize);
            }
            Node::Data { tag } => tag.evaluate(self, node_id)?,
            _ => {}
        }

        Ok(())
    }
}

impl AST {
    pub fn add_debug_frame_with_annotation(&mut self, id: NodeIndex, text: &str) {
        let node = self.graph.add_node(Node::Debug(DebugNode::Annotation {
            text: text.to_string(),
        }));
        let edge = self.graph.add_edge(node, id, Edge::Debug);
        self.add_debug_frame();
        self.graph.remove_node(node);
        self.graph.remove_edge(edge);
    }
    pub fn add_debug_frame(&mut self) {
        if false {
            self.debug_frames.push(self.to_dot());
        }
    }
    pub fn dump_debug(&self) {
        let mut seen = HashSet::new();

        for (id, frame) in self
            .debug_frames
            .iter()
            .filter(|frame| seen.insert(*frame))
            .enumerate()
        {
            std::fs::write(format!("./ast-{:04}.dot", id), frame).unwrap();
        }
    }
    fn check_variable_integrity(&mut self, node_id: NodeIndex) {
        match self.graph.node_weight(node_id) {
            Some(Node::Variable {
                name,
                kind: VariableKind::Bound { depth },
            }) => {
                let binding_closure_id = self.find_closure_at_depth(node_id, *depth).unwrap();
                match self.graph.node_weight(binding_closure_id) {
                    Some(Node::Closure { argument_name } | Node::Lambda { argument_name }) => {
                        let argument_name = argument_name.clone().to_string();
                        let name = name.clone().to_string();
                        if argument_name != name {
                            self.add_debug_frame();
                            panic!(
                                "Expected {name}, got {argument_name} at {} (binding closure at {})",
                                node_id.index(),
                                binding_closure_id.index()
                            )
                        }
                    }
                    _ => panic!(),
                }
            }
            _ => {}
        }
    }
    fn integrity_check(&mut self, under_id: NodeIndex) {
        let mut traverser = LambdaDepthTraverser::new(under_id);
        while let Some((index, _)) = traverser.next(&self.graph) {
            self.check_variable_integrity(index);
        }
    }
}

impl Display for AST {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.fmt_expr(self.root, 0).map_err(|_| std::fmt::Error)?
        )
    }
}
