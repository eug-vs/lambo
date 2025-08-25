use crate::{
    parser::{lexer::lexer, parser::parse_expr},
    Expr,
};

mod lexer;
mod parser;

impl Expr {
    pub fn from_str(s: &str) -> Expr {
        parse_expr(&mut lexer(s).peekable(), 0, vec![])
    }
}

#[cfg(test)]
mod tests {
    use crate::Expr;

    #[test]
    fn parse_basic() {
        let s = "func (#eq a b) λx.λy.x";
        let expr = Expr::from_str(s);
        assert_eq!(format!("{}", expr), "((func ((#eq a) b)) λx.λy.x)");
        assert_eq!(
            format!("{}", expr.fmt_de_brujin()),
            "((func ((#eq a) b)) λ λ 2)"
        );
    }
}
