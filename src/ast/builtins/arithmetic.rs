use petgraph::graph::NodeIndex;

use crate::ast::{
    builtins::ConstructorTag, ASTError, ASTResult, Edge, Node, Number, Primitive, AST,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArithmeticTag {
    Add,
    Mul,
    Pow,
    Sub,
    Div,
    Eq,
}

impl Primitive {
    pub fn extract_number(&self) -> ASTResult<Number> {
        match self {
            Primitive::Number(number) => ASTResult::Ok(*number),
        }
    }
}

impl ArithmeticTag {
    pub fn argument_names(&self) -> Vec<&'static str> {
        vec!["what", "to"]
    }

    pub fn evaluate(&self, ast: &mut AST, id: NodeIndex) -> ASTResult<NodeIndex> {
        // All arithmetic is strict in all parameters
        let [what, to] = ConstructorTag::get_binders(ast, id)
            .iter()
            .map(|&binder| {
                ast.extract_primitive_from_environment(binder)
                    .map(|p| p.extract_number())
                    .flatten()
            })
            .collect::<ASTResult<Vec<_>>>()?
            .try_into()
            .expect("Incorrect argument count for arithmetic operation");

        let result = match self {
            Self::Eq => {
                let result =
                    ast.add_expr_from_str(if what == to { "λx.λy.x" } else { "λx.λy.y" });
                ast.migrate_node(id, result);
                ast.remove_subtree(id);
                return Ok(result);
            }
            Self::Add => what + to,
            Self::Mul => what * to,
            Self::Pow => to.pow(what as u32),
            Self::Sub => to.checked_sub(what).unwrap_or_default(),
            Self::Div => to / what,
        };
        let result = ast
            .graph
            .add_node(Node::Primitive(Primitive::Number(result)));
        ast.migrate_node(id, result);
        ast.remove_subtree(id);
        Ok(result)
    }
}
