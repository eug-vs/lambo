use std::{
    fmt::{Debug, Display},
    rc::Rc,
};

use smallvec::SmallVec;

use builtins::BuiltinFunctionDeclaration;

pub mod builtins;
mod debug;
mod reduction;
mod strong;

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

#[derive(Debug, Clone, Copy)]
pub enum IO {
    PutChar(usize),
    GetChar,
    Throw,
}

#[derive(Debug, Clone)]
pub enum DataValue {
    Number(usize),
    IO(IO),
}

#[derive(Debug, Clone)]
pub enum Node {
    Var {
        name: String,
        kind: VariableKind,
    },
    Lambda {
        argument: String,
        body: usize,
    },
    Call {
        function: usize,
        parameter: usize,
    },
    /// Token represent a *body* of builtin function
    Token {
        declaration: Rc<BuiltinFunctionDeclaration>,
        /// Childen - bound variables
        variables: Vec<usize>,
    },
    Data(DataValue),
    Consumed(String),
}

#[derive(Clone)]
pub struct Graph {
    pub graph: SmallVec<[Node; 1024]>,
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

    pub fn add_builtin(&mut self, declaration: Rc<BuiltinFunctionDeclaration>) -> usize {
        let variables = declaration
            .argument_names
            .iter()
            .rev()
            .enumerate()
            .map(|(index, name)| {
                self.add_node(Node::Var {
                    name: name.clone(),
                    kind: VariableKind::Bound { depth: index + 1 },
                })
            })
            .rev()
            .collect();

        let mut id = self.add_node(Node::Token {
            declaration: declaration.clone(),
            variables,
        });
        for argument in declaration.argument_names.iter().rev() {
            id = self.add_node(Node::Lambda {
                argument: argument.clone(),
                body: id,
            });
        }
        id
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
            Node::Token { declaration, .. } => format!("TOKEN_{}", declaration.name),
            Node::Data(value) => format!("{:?}", value),
            Node::Consumed(_) => self.panic_consumed_node(expr),
        }
    }
    pub fn fmt_expr(&self, expr: usize) -> String {
        match &self.graph[expr] {
            Node::Var { name, .. } => name.to_string(),
            Node::Lambda { body, argument } => format!("λ{}.{}", argument, self.fmt_expr(*body)),
            Node::Call {
                function,
                parameter,
                ..
            } => format!(
                "({} {})",
                self.fmt_expr(*function),
                self.fmt_expr(*parameter)
            ),
            Node::Token { declaration, .. } => format!("TOKEN_{}", declaration.name),
            Node::Data(value) => format!("{:?}", value),
            Node::Consumed(_) => self.panic_consumed_node(expr),
        }
    }

    pub fn size(&self) -> usize {
        self.graph.len()
    }
}

impl Display for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_expr(self.root))
    }
}
