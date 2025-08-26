use std::fmt::Display;

use smallvec::SmallVec;

mod debug;
mod reduction;

#[derive(Debug, Clone)]
pub enum VariableKind {
    /// Represents a De Brujin index
    Bound {
        depth: usize,
    },
    Free,
}

#[derive(Debug, Clone)]
pub enum DebugConfig {
    Enabled {
        dump_path: String,
        auto_dump_every: usize,
    },
    Disabled,
}

#[derive(Debug, Clone)]
pub enum Node {
    Var { name: String, kind: VariableKind },
    Lambda { argument: String, body: usize },
    Call { function: usize, parameter: usize },
    Consumed(String),
}

#[derive(Debug)]
pub struct Graph {
    graph: SmallVec<[Node; 1024]>,
    pub root: usize,
    debug_config: DebugConfig,
    debug_frames: SmallVec<[String; 1024]>,
    debug_last_dump_at: usize,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            graph: SmallVec::new(),
            root: 0,
            debug_config: DebugConfig::Disabled,
            debug_frames: SmallVec::new(),
            debug_last_dump_at: 0,
        }
    }

    pub fn add_node(&mut self, node: Node) -> usize {
        self.graph.push(node);
        self.graph.len() - 1
    }

    fn panic_consumed_node(&self, id: usize) -> ! {
        self.dump_debug_frames();
        panic!("Tried to access a dead node: {id}");
    }

    pub fn fmt_de_brujin(&self, expr: usize) -> String {
        match &self.graph[expr] {
            Node::Var {
                name,
                kind: VariableKind::Free,
                ..
            } => name.to_string(),
            Node::Var {
                kind: VariableKind::Bound { depth, .. },
                ..
            } => format!("{}", depth),
            Node::Lambda { body, .. } => format!("λ {}", self.fmt_de_brujin(*body)),
            Node::Call {
                function,
                parameter,
                ..
            } => format!(
                "({} {})",
                self.fmt_de_brujin(*function),
                self.fmt_de_brujin(*parameter)
            ),
            Node::Consumed(_) => self.panic_consumed_node(expr),
        }
    }
    fn fmt_expr(&self, f: &mut std::fmt::Formatter<'_>, expr: usize) -> std::fmt::Result {
        match &self.graph[expr] {
            Node::Var { name, .. } => write!(f, "{}", name),
            Node::Lambda { argument, body, .. } => {
                write!(f, "λ{}", argument)?;
                write!(f, ".")?;
                self.fmt_expr(f, *body)
            }
            Node::Call {
                function,
                parameter,
                ..
            } => {
                write!(f, "(")?;
                match self.graph[*function] {
                    Node::Lambda { .. } => {
                        write!(f, "(")?;
                        self.fmt_expr(f, *function)?;
                        write!(f, ")")?;
                    }
                    _ => self.fmt_expr(f, *function)?,
                }
                write!(f, " ")?;

                // We can avoid wrapping argument in (),
                // even if it's a Lambda, because closing paren follows anyway
                self.fmt_expr(f, *parameter)?;
                write!(f, ")")
            }
            Node::Consumed(_) => self.panic_consumed_node(expr),
        }
    }
}

impl Display for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_expr(f, self.root)
    }
}
