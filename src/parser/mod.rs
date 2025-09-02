use crate::{
    evaluator::{builtins::BuiltinFunctionRegistry, Graph},
    parser::{lexer::lexer, parser::parse_expr},
};

mod lexer;
mod parser;

impl Graph {
    pub fn from_str(s: &str, registry: &BuiltinFunctionRegistry) -> Self {
        let mut graph = Self::new();
        graph.root = parse_expr(&mut graph, &mut lexer(s).peekable(), 0, vec![], registry);
        graph
    }
    pub fn add_expr_from_str(&mut self, s: &str) -> usize {
        parse_expr(
            self,
            &mut lexer(s).peekable(),
            0,
            vec![],
            &BuiltinFunctionRegistry::new(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::evaluator::Graph;

    #[test]
    fn parse_basic() {
        let s = "func (#eq a b) λx.λy.x";
        let expr = Graph::from_str(s, &HashMap::new());
        assert_eq!(format!("{}", expr), "((func ((#eq a) b)) λx.λy.x)");
        assert_eq!(
            format!("{}", expr.fmt_de_brujin(expr.root)),
            "((func ((#eq a) b)) λ λ 2)"
        );
    }
}
