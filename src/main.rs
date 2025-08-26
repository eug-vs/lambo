use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::evaluator::{DebugConfig, Graph};
mod evaluator;
mod parser;

fn extract_from_markdown() -> Vec<String> {
    let input = fs::read_to_string("./README.md").unwrap();
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for line in input.lines() {
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            lines.push(line.to_string());
        }
    }

    lines
}

fn main() {
    let mut context = vec![];

    let extracted = extract_from_markdown();
    let mut lines = extracted
        .iter()
        .map(|line| line.split("//").next().unwrap_or(""))
        .filter(|line| !line.trim().is_empty());

    while let Some(line) = lines.next() {
        let input = {
            // Handle multiline statements with [ ]
            if line.contains("[") {
                let mut joined = line.to_string().replace("[", "");
                for l in lines.by_ref() {
                    joined = joined + "\n" + &l.replace("]", "");
                    if l.contains("]") {
                        break;
                    }
                }
                joined
            } else {
                line.to_string()
            }
        };
        let mut words = input.split(&[' ', '\t']).peekable();
        match words.peek().unwrap() {
            &"let" => {
                words.next();
                let variable_name = words.next().unwrap();
                let expr_string = &words.collect::<Vec<_>>().join(" ");
                context.push(format!("with {} {} in", variable_name, expr_string));
            }
            _ => {
                let input = &words.collect::<Vec<_>>().join(" ");
                println!();
                println!("$   {}", input);
                let mut graph =
                    Graph::from_str(format!("{} {}", context.join(" "), input).as_str());

                let dump_path = {
                    let mut hasher = DefaultHasher::new();
                    graph.fmt_de_brujin(graph.root).hash(&mut hasher);
                    let hash = hasher.finish();
                    format!("./debug/{}", hash)
                };
                graph.enable_debug(DebugConfig::Enabled {
                    dump_path,
                    auto_dump_every: 100,
                });

                graph.evaluate(graph.root, &mut vec![]);
                let root = graph.unwrap_closure_chain(graph.root, vec![]);
                graph.root = root;
                println!(" => {}", graph);
                println!("||  {} nodes", graph.size());

                graph.dump_debug_frames();
            }
        }
    }
}
