use petgraph::graph::NodeIndex;

use crate::ast::{ASTError, ASTResult, Edge, Node, Number, Primitive, AST};

#[derive(Debug, Clone, Copy)]
pub enum ArithmeticTag {
    Add,
    Mul,
    Pow,
    Sub,
    Div,
    Eq,
}

impl ArithmeticTag {
    pub fn argument_names(&self) -> Vec<&'static str> {
        vec!["what", "to"]
    }

    fn extract_number(ast: &mut AST, id: NodeIndex) -> ASTResult<Number> {
        match ast.graph.node_weight(id) {
            Some(Node::Primitive(Primitive::Number(number))) => ASTResult::Ok(*number),
            _ => ASTResult::Err(ASTError::Custom(id, "NaN")),
        }
    }

    pub fn evaluate(&self, ast: &mut AST, id: NodeIndex) -> ASTResult<NodeIndex> {
        let [what, to] = [0, 1]
            .iter()
            .map(|&argument_index| {
                // All arithmetic is strict in all parameters
                ast.evaluate(ast.follow_edge(id, Edge::ConstructorArgument(argument_index))?)?;
                ArithmeticTag::extract_number(
                    ast,
                    ast.follow_edge(id, Edge::ConstructorArgument(argument_index))?,
                )
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("Incorrect argument count for arithmetic operation");

        let what = what?;
        let to = to?;

        let result = match self {
            Self::Eq => {
                let result =
                    ast.add_expr_from_str(if what == to { "λx.λy.x" } else { "λx.λy.y" });
                ast.migrate_node(id, result);
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
        Ok(result)
    }
}
