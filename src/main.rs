use crate::ast::AST;
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

            // Strip comments
            input = input
                .lines()
                .map(|line| line.split("//").next().unwrap())
                .collect::<Vec<_>>()
                .join("\n");

            let mut ast = AST::from_str(&input);
            println!(" $\n{}", ast);
            ast.add_debug_frame();

            if ENABLE_TRACING {
                setup_global_subscriber();
            }

            if let Err(err) = ast.evaluate(ast.root) {
                ast.debug_ast_error(err)
            };
            ast.add_debug_frame();
            ast.dump_debug();
            println!(" >\n{}", ast);
        })
        .unwrap();

    child.join().unwrap();
}
