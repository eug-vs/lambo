use crate::{
    evaluator::Graph,
    parser::{lexer::lexer, parser::parse_expr},
};

mod lexer;
mod parser;

impl Graph {
    pub fn from_str(s: &str) -> Self {
        let mut graph = Self::new();
        graph.root = parse_expr(&mut graph, &mut lexer(s).peekable(), 0, vec![]);
        graph
    }
}

#[cfg(test)]
mod tests {
    use crate::evaluator::Graph;

    #[test]
    fn parse_basic() {
        let s = "func (#eq a b) λx.λy.x";
        let expr = Graph::from_str(s);
        assert_eq!(format!("{}", expr), "((func ((#eq a) b)) λx.λy.x)");
        assert_eq!(
            format!("{}", expr.fmt_de_brujin(expr.root)),
            "((func ((#eq a) b)) λ λ 2)"
        );
    }
}
