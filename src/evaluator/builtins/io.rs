use std::{io::stdin, rc::Rc};

use crate::evaluator::{
    builtins::{BuiltinFunctionDeclaration, BuiltinFunctionRegistry},
    DataValue, Graph, Node, VariableKind, IO,
};

impl Graph {
    pub fn run_io(&mut self, io: IO) -> usize {
        match io {
            IO::GetChar => {
                let mut line = String::new();
                stdin().read_line(&mut line).unwrap();
                let number = line.trim().parse().unwrap();

                self.add_node(Node::Data(DataValue::Number(number)))
            }
            IO::PutChar(char_code) => {
                print!("{}", char::from_u32(char_code as u32).unwrap());
                self.add_node(Node::Var {
                    name: "#PUTCHAR_FINISHED".to_string(),
                    kind: VariableKind::Free,
                })
            }
            IO::Throw => {
                panic!("#io_throw was called")
            }
        }
    }
}

pub fn register_io(registry: &mut BuiltinFunctionRegistry) {
    registry.insert(
        "#io_getchar".to_string(),
        Rc::new(BuiltinFunctionDeclaration {
            name: "#io_getchar".to_string(),
            argument_names: vec![],
            to_value: Box::new(move |graph, _| {
                graph.add_node(Node::Data(DataValue::IO(IO::GetChar)))
            }),
        }),
    );
    registry.insert(
        "#io_putchar".to_string(),
        Rc::new(BuiltinFunctionDeclaration {
            name: "#io_putchar".to_string(),
            argument_names: vec!["char_code".to_string()],
            to_value: Box::new(move |graph, argument_ids| {
                let node = argument_ids[0];
                let char_code = match graph.graph[node] {
                    Node::Data(DataValue::Number(number)) => number,
                    _ => panic!("Expected Number, got {:?}", graph.graph[node]),
                };
                graph.add_node(Node::Data(DataValue::IO(IO::PutChar(char_code))))
            }),
        }),
    );
    registry.insert(
        "#io_throw".to_string(),
        Rc::new(BuiltinFunctionDeclaration {
            name: "#io_throw".to_string(),
            argument_names: vec!["unused_param".to_string()],
            to_value: Box::new(move |graph, _| {
                graph.add_node(Node::Data(DataValue::IO(IO::Throw)))
            }),
        }),
    );

    registry.insert(
        "#io_flatmap".to_string(),
        Rc::new(BuiltinFunctionDeclaration {
            name: "#io_flatmap".to_string(),
            argument_names: vec!["transform".to_string(), "io".to_string()],
            to_value: Box::new(move |graph, argument_ids| -> usize {
                graph.add_debug_frame(vec![]);
                graph.dump_debug_frames();
                let io = match &graph.graph[argument_ids[1]] {
                    Node::Data(DataValue::IO(io)) => io,
                    _ => panic!("Expected IO"),
                };
                let io_result = graph.run_io(*io);
                graph.add_node(Node::Call {
                    function: argument_ids[0],
                    parameter: io_result,
                })
            }),
        }),
    );
}
