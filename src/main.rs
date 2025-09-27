use std::{
    io::{stdin, Read},
    thread,
};

use crate::ast::AST;

mod ast;
mod parser;

fn main() {
    let child = thread::Builder::new()
        // Increase stack size
        .stack_size(1024 * 1024 * 40)
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
