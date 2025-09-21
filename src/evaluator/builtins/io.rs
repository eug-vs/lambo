use std::io::stdin;

use crate::evaluator::{
    builtins::ConstructorTag, reduction::ClosurePath, Graph, Node, Primitive, VariableKind,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum IOTag {
    GetChar,
    PutChar,
    Throw,
    Flatmap,
}

impl IOTag {
    pub fn argument_names(&self) -> Vec<&'static str> {
        match self {
            IOTag::Throw => vec![],
            IOTag::GetChar => vec![],
            IOTag::PutChar => vec!["char_code"],
            IOTag::Flatmap => vec!["transform", "io"],
        }
    }

    pub fn flatmap(
        graph: &mut Graph,
        closure_path: &mut ClosurePath,
        arguments: Vec<usize>,
    ) -> usize {
        let transform = arguments[0];
        let io = arguments[1];

        graph.evaluate(io, closure_path);

        let io_result = match &graph.graph[io] {
            Node::Data {
                tag: ConstructorTag::IO(io_tag),
                constructor_params,
            } => match io_tag {
                IOTag::GetChar => {
                    let mut line = String::new();
                    stdin().read_line(&mut line).unwrap();
                    let number = line.trim().parse().unwrap();

                    graph.add_node(Node::Primitive(Primitive::Number(number)))
                }
                IOTag::PutChar => {
                    let char_code_id = constructor_params[0];
                    graph.evaluate(char_code_id, closure_path);
                    match graph.graph[char_code_id] {
                        Node::Primitive(Primitive::Number(char_code)) => {
                            print!("{}", char::from_u32(char_code as u32).unwrap());
                            graph.add_node(Node::Var {
                                name: "#PUTCHAR_FINISHED".to_string(),
                                kind: VariableKind::Free,
                            })
                        }
                        _ => panic!("Expected number for charcode"),
                    }
                }
                IOTag::Throw => {
                    panic!("#io_throw was called explicitly")
                }
                IOTag::Flatmap => panic!("#io_flatmap is not an effectful IO"),
            },
            node => panic!("Expected IO, found {:?}", node),
        };

        let result = graph.add_node(Node::Call {
            function: transform,
            parameter: io_result,
        });
        // Evaluate transform function with the result of IO
        graph.evaluate(result, closure_path);
        result
    }
}
