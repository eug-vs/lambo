use petgraph::graph::NodeIndex;

use crate::{
    ast::AST,
    parser::{lexer::lexer, parser::parse_expr},
};

mod lexer;
mod parser;

impl AST {
    pub fn from_str(s: &str) -> Self {
        let mut ast = Self::new();
        ast.root = parse_expr(&mut ast, &mut lexer(s).peekable(), 0, vec![]);
        ast
    }
    pub fn add_expr_from_str(&mut self, s: &str) -> NodeIndex {
        parse_expr(self, &mut lexer(s).peekable(), 0, vec![])
        // unimplemented!("Please provide reference to parent environment");
    }
}
