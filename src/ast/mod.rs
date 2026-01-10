use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    rc::Rc,
};

pub mod builtins;
mod debug;

use petgraph::{
    graph::{EdgeIndex, NodeIndex},
    prelude::StableGraph,
    stable_graph::EdgeReference,
    visit::EdgeRef,
    Direction,
};

use crate::ast::builtins::ConstructorTag;

#[derive(Debug, Clone)]
pub enum VariableKind {
    Free(Rc<String>),
    Bound,
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
    Binder,
    Debug,
    ConstructorArgument(usize),
}

#[derive(Debug, Clone)]
pub enum Node {
    Lambda { argument_name: Rc<String> },
    Application,
    Variable(VariableKind),
    Primitive(Primitive),
    Closure { argument_name: Rc<String> },
    Data { tag: ConstructorTag },
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

        for edge in graph
            .edges_directed(id, Direction::Outgoing)
            .filter(|e| !matches!(e.weight(), Edge::Binder))
        {
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
    #[tracing::instrument(skip(self))]
    fn follow_edge(&self, expr: NodeIndex, edge: Edge) -> ASTResult<NodeIndex> {
        self.get_edge_ref(expr, edge).map(|e| e.target())
    }
    #[tracing::instrument(skip(self))]
    fn redirect_edge(&mut self, edge_id: EdgeIndex, node: NodeIndex) {
        let (source, _) = self.graph.edge_endpoints(edge_id).unwrap();
        let edge = self.graph.remove_edge(edge_id).unwrap();
        self.graph.add_edge(source, node, edge);
    }
    #[tracing::instrument(skip(self))]
    fn migrate_node(&mut self, from: NodeIndex, to: NodeIndex) {
        for edge in self
            .graph
            .edges_directed(from, Direction::Incoming)
            .filter(|e| !matches!(e.weight(), Edge::Binder))
            .map(|e| e.id())
            .collect::<Vec<_>>()
        {
            self.redirect_edge(edge, to)
        }

        if self.root == from {
            self.root = to;
        }
    }
    pub fn get_variable_name(&self, id: NodeIndex) -> ASTResult<&String> {
        match self.graph.node_weight(id).unwrap() {
            Node::Variable(VariableKind::Free(name)) => Ok(name),
            Node::Variable(VariableKind::Bound) => {
                let binder_id = self.follow_edge(id, Edge::Binder)?;
                if let Some(Node::Closure { argument_name } | Node::Lambda { argument_name }) =
                    self.graph.node_weight(binder_id)
                {
                    Ok(argument_name)
                } else {
                    Err(ASTError::Custom(id, "Incorrect binder"))
                }
            }
            _ => Err(ASTError::Custom(id, "Not a variable")),
        }
    }
    pub fn fmt_expr(&self, expr: NodeIndex, tab_index: usize) -> ASTResult<String> {
        let indent = "  ".repeat(tab_index);
        match &self.graph[expr] {
            Node::Variable(_) => Ok(self.get_variable_name(expr)?.to_string()),
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
    #[tracing::instrument(skip(self))]
    fn clone_subtree(
        &mut self,
        node_id: NodeIndex,
        mut binder_remaps: HashMap<NodeIndex, NodeIndex>,
    ) -> NodeIndex {
        let node_weight = self.graph.node_weight(node_id).unwrap().clone();
        let is_binder = matches!(node_weight, Node::Closure { .. } | Node::Lambda { .. });
        let cloned_id = self.graph.add_node(node_weight);

        if is_binder {
            binder_remaps.insert(node_id, cloned_id);
        }

        let edges = self
            .graph
            .edges_directed(node_id, Direction::Outgoing)
            .map(|e| (e.target(), *e.weight()))
            .collect::<Vec<_>>();

        for (target, weight) in edges {
            let to = match weight {
                Edge::Binder => *binder_remaps.get(&target).unwrap_or(&target),
                _ => self.clone_subtree(target, binder_remaps.clone()),
            };
            self.graph.add_edge(cloned_id, to, weight);
        }
        cloned_id
    }

    /// Lifts environment above the current node and returns the length of lifted closure chain
    #[tracing::instrument(skip(self))]
    fn lift_closure_chain(
        &mut self,
        node_id: NodeIndex,
        node_under_closures: NodeIndex,
        edge: Edge,
    ) -> ASTResult<()> {
        assert!(
            !matches!(
                self.graph.node_weight(node_under_closures).unwrap(),
                Node::Closure { .. }
            ),
            "Node under closures can't itself be a closure"
        );
        let (edge_id, edge_target) = self
            .get_edge_ref(node_id, edge)
            .map(|edge_ref| (edge_ref.id(), edge_ref.target()))?;

        if let Node::Closure { .. } = self.graph.node_weight(edge_target).unwrap() {
            let first_closure = edge_target;
            // Parent now points to a closure chain
            self.migrate_node(node_id, first_closure);

            // Closure chain now points to current node
            self.migrate_node(node_under_closures, node_id);

            // Current edge now points to whatever was under closure chain
            self.redirect_edge(edge_id, node_under_closures);

            self.add_debug_frame();
        }

        Ok(())
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

    /// Returns NodeIndex under the closure chain
    pub fn evaluate(&mut self, node_id: NodeIndex) -> Result<NodeIndex, ASTError> {
        self.add_debug_frame_with_annotation(node_id, "evaluate");
        match *self.graph.node_weight(node_id).unwrap() {
            Node::Closure { .. } => {
                let body = self.follow_edge(node_id, Edge::Body)?;
                return self.evaluate(body);
            }
            Node::Application => {
                let under_closures = self.evaluate(self.follow_edge(node_id, Edge::Function)?)?;
                self.lift_closure_chain(node_id, under_closures, Edge::Function)?;

                let function_target = self.follow_edge(node_id, Edge::Function)?;

                if let Node::Lambda { argument_name } =
                    self.graph.node_weight(function_target).unwrap()
                {
                    let argument_name = argument_name.clone();

                    // Lambda node becomes a closure
                    self.migrate_node(node_id, function_target);
                    *self.graph.node_weight_mut(function_target).unwrap() =
                        Node::Closure { argument_name };
                    let closure_id = function_target;

                    // Add parameter edge to the closure
                    let parameter_target = self.follow_edge(node_id, Edge::Parameter)?;
                    self.graph
                        .add_edge(closure_id, parameter_target, Edge::Parameter);

                    // Cleanup application node
                    self.graph.remove_node(node_id);

                    return self.evaluate(closure_id);
                }
            }
            Node::Variable(VariableKind::Bound) => {
                let binding_closure_id = self.follow_edge(node_id, Edge::Binder)?;
                let under_closures =
                    self.evaluate(self.follow_edge(binding_closure_id, Edge::Parameter)?)?;
                self.lift_closure_chain(binding_closure_id, under_closures, Edge::Parameter)?;

                let cloned_node_id = self.clone_subtree(
                    self.follow_edge(binding_closure_id, Edge::Parameter)?,
                    HashMap::new(),
                );
                self.migrate_node(node_id, cloned_node_id);
                self.graph.remove_node(node_id);
                return Ok(cloned_node_id);
            }
            Node::Data { tag } => return tag.evaluate(self, node_id),
            _ => {}
        }

        Ok(node_id)
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
