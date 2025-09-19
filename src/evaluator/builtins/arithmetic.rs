use std::rc::Rc;

use crate::evaluator::{
    builtins::{BuiltinFunctionDeclaration, BuiltinFunctionRegistry},
    DataValue, Graph, Node, VariableKind,
};

impl Graph {
    fn count_calls(&self, expr: usize) -> usize {
        // TODO: validate depth
        match self.graph[expr] {
            Node::Call { parameter, .. } => 1 + self.count_calls(parameter),
            Node::Var {
                kind: VariableKind::Bound { .. },
                ..
            } => 0,
            _ => panic!("Invalid number"),
        }
    }
    fn parse_church_number(&self, expr: usize) -> usize {
        match &self.graph[expr] {
            Node::Lambda { body, .. } => match &self.graph[*body] {
                Node::Lambda { body, .. } => self.count_calls(*body),
                _ => {
                    self.dump_debug_frames();
                    panic!("Not a number");
                }
            },
            Node::Var {
                name,
                kind: VariableKind::Free,
            } => name.parse().unwrap(),
            _ => panic!("Not a number"),
        }
    }
}

pub fn register_arithmetic(registry: &mut BuiltinFunctionRegistry) {
    let ops: Vec<(&str, Box<dyn Fn(&mut Graph, usize, usize) -> usize>)> = vec![
        (
            "+",
            Box::new(|graph, a, b| graph.add_node(Node::Data(DataValue::Number(a + b)))),
        ),
        (
            "*",
            Box::new(|graph, a, b| graph.add_node(Node::Data(DataValue::Number(a * b)))),
        ),
        // Reverse ordergraph,  for "inverse" operations, to have e.g (- 5) as valid function
        (
            "^",
            Box::new(|graph, a, b| graph.add_node(Node::Data(DataValue::Number(b.pow(a as u32))))),
        ),
        (
            "-",
            Box::new(|graph, a, b| {
                graph.add_node(Node::Data(DataValue::Number(
                    b.checked_sub(a).unwrap_or_default(),
                )))
            }),
        ),
        (
            "/",
            Box::new(|graph, a, b| graph.add_node(Node::Data(DataValue::Number(b / a)))),
        ),
        (
            "=num",
            Box::new(|graph, a, b| {
                graph.add_expr_from_str(if a == b { "λx.λy.x" } else { "λx.λy.y" })
            }),
        ),
    ];

    for (name, func) in ops {
        registry.insert(
            name.to_string(),
            Rc::new(BuiltinFunctionDeclaration {
                name: name.to_string(),
                argument_names: vec!["a".to_string(), "b".to_string()],
                to_value: Box::new(move |graph, argument_ids| -> usize {
                    let [a, b] = argument_ids[0..2]
                        .iter()
                        .map(|node| match graph.graph[*node] {
                            Node::Data(DataValue::Number(number)) => number,
                            _ => {
                                graph.add_debug_frame(vec![(*node, "Expected number")]);
                                graph.dump_debug_frames();
                                panic!(
                                    "Expected Number while doing {name}, got {:?}",
                                    graph.graph[*node]
                                )
                            }
                        })
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap();

                    func(graph, a, b)
                }),
            }),
        );
    }
}
