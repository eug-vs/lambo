use crate::evaluator::{reduction::ClosurePath, Graph, Node, Primitive};

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
    pub fn evaluate(
        &self,
        graph: &mut Graph,
        closure_path: &mut ClosurePath,
        arguments: Vec<usize>,
    ) -> usize {
        let [what, to] = arguments
            .iter()
            .map(|node| {
                // All arithmetic is strict in all parameters
                graph.evaluate(*node, closure_path);
                graph.graph[*node]
                    .extract_primitive_number()
                    .expect("Expected number for arithmetic operation")
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("Incorrect argument count for arithmetic operation");

        let result = match self {
            Self::Eq => {
                return graph.add_expr_from_str(if what == to { "λx.λy.x" } else { "λx.λy.y" })
            }

            Self::Add => what + to,
            Self::Mul => what * to,
            Self::Pow => to.pow(what as u32),
            Self::Sub => to.checked_sub(what).unwrap_or_default(),
            Self::Div => to / what,
        };
        graph.add_node(Node::Primitive(Primitive::Number(result)))
    }
}
