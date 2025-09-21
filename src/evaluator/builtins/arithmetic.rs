use crate::evaluator::{reduction::ClosurePath, Graph, Node, Primitive, VariableKind};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
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
        let [what, to] = arguments[0..2]
            .iter()
            .map(|node| {
                // All arithmetic is strict
                graph.evaluate(*node, closure_path);
                match graph.graph[*node] {
                    Node::Primitive(Primitive::Number(number)) => number,
                    _ => {
                        graph.add_debug_frame(vec![(*node, "Expected number")]);
                        graph.dump_debug_frames();
                        panic!(
                            "Expected Number while doing {:?}, got {:?}",
                            self, graph.graph[*node]
                        )
                    }
                }
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

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
