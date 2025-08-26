use std::mem;

use smallvec::SmallVec;

use crate::{Expr, VariableKind};

mod debug;
mod reduction;

#[derive(Debug, Clone)]
enum Node {
    Var { name: String, kind: VariableKind },
    Lambda { argument: String, body: usize },
    Call { function: usize, parameter: usize },
    Consumed(String),
}

#[derive(Debug)]
pub struct Graph {
    graph: SmallVec<[Node; 1024]>,
    pub root: usize,
    debug: bool,
    debug_frames: SmallVec<[String; 1024]>,
}

impl Graph {
    fn new(debug: bool) -> Self {
        Graph {
            graph: SmallVec::new(),
            root: 0,
            debug,
            debug_frames: SmallVec::new(),
        }
    }
    /// Consume Expr to create graph
    pub fn from_expr(expr: Expr, debug: bool) -> Self {
        let mut graph = Self::new(debug);
        graph.root = graph.add_expr_to_graph(expr);
        graph
    }

    fn add_expr_to_graph(&mut self, expr: Expr) -> usize {
        match expr {
            Expr::Var { name, kind, .. } => {
                self.graph.push(Node::Var { name, kind });
            }
            Expr::Lambda { body, argument, .. } => {
                let body_id = self.add_expr_to_graph(*body);
                self.graph.push(Node::Lambda {
                    argument,
                    body: body_id,
                });
            }
            Expr::Call {
                function,
                parameter,
                ..
            } => {
                let function_id = self.add_expr_to_graph(*function);
                let parameter_id = self.add_expr_to_graph(*parameter);
                self.graph.push(Node::Call {
                    function: function_id,
                    parameter: parameter_id,
                });
            }
        };
        self.graph.len() - 1
    }

    pub fn to_expr(&mut self, id: usize) -> Expr {
        let node = mem::replace(&mut self.graph[id], Node::Consumed("to_expr".to_string()));
        match node {
            Node::Var { name, kind } => Expr::Var { name, kind },
            Node::Lambda { argument, body } => Expr::Lambda {
                argument,
                body: Box::new(self.to_expr(body)),
            },
            Node::Call {
                function,
                parameter,
            } => Expr::Call {
                function: Box::new(self.to_expr(function)),
                parameter: Box::new(self.to_expr(parameter)),
            },
            Node::Consumed(_) => self.panic_consumed_node(id),
        }
    }

    fn panic_consumed_node(&self, id: usize) -> ! {
        self.dump_debug_frames("./error");
        panic!("Tried to access a dead node: {id}");
    }
}
