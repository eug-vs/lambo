use crate::ast::{builtins::ConstructorTag, Node, AST};
use std::{
    io::{stdin, Read},
    thread,
};
use tracing_flame::FlameLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, registry::Registry};

fn setup_global_subscriber() -> impl Drop {
    let fmt_layer = fmt::Layer::default();

    let (flame_layer, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();

    let subscriber = Registry::default().with(fmt_layer).with(flame_layer);
    // .with(HierarchicalLayer::new(2).with_ansi(true));

    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");

    _guard
}

pub mod ast;
pub mod parser;

const ENABLE_TRACING: bool = false;

fn main() {
    let child = thread::Builder::new()
        // Increase stack size
        .stack_size(1024 * 1024 * 100)
        .spawn(|| {
            let mut input = String::new();
            stdin().read_to_string(&mut input).unwrap();

            let mut ast = AST::from_str(&input);
            ast.garbage_collect();
            println!(" $\n{}", ast);
            ast.add_debug_frame();

            if ENABLE_TRACING {
                setup_global_subscriber();
            }

            if let Err(err) = ast.evaluate(ast.root) {
                ast.debug_ast_error(err)
            };
            ast.garbage_collect();

            match ast.graph.node_weight(ast.root).unwrap() {
                &Node::Data {
                    tag: ConstructorTag::IO(io),
                } => {
                    let root = ast.root;
                    io.run(&mut ast, root).unwrap();
                }
                _ => {}
            }

            ast.add_debug_frame();
            ast.dump_debug();
            println!(" >\n{}", ast);
        })
        .unwrap();

    child.join().unwrap();
}
