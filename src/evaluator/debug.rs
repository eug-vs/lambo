use std::{collections::HashSet, fs, iter::once};

use crate::{
    evaluator::{DebugConfig, Node, VariableKind},
    Graph,
};

impl Graph {
    /// Convert current Graph state into String in DOT format.
    /// This can be then rendered using graphviz into PNG/SVG for analysis and debugging.
    /// You can pass a list of additional labels for nodes.
    /// Also see debug.html for interactive DOT viewer
    pub fn to_dot(&self, labels: Vec<(usize, &str)>) -> String {
        use std::fmt::Write;

        let mut result = String::from("digraph EXPR {\n");
        let root_label = (self.root, "ROOT");

        let mut subtree = HashSet::new();
        for (label_id, (node_id, name)) in once(&root_label).chain(labels.iter()).enumerate() {
            writeln!(result, "LABEL{label_id} [label=\"{name}\", color=\"red\"]",).unwrap();
            writeln!(result, "LABEL{label_id} -> {}", node_id).unwrap();

            // Helpers
            // if *name != "ROOT" {
            //     writeln!(
            //         result,
            //         "HELPER{label_id} [label=\"{}\", color=\"green\"]",
            //         self.fmt_expr(*node_id)
            //     )
            //     .unwrap();
            //     writeln!(result, "HELPER{label_id} -> {}", node_id).unwrap();
            // }

            subtree.extend(self.traverse_subtree(*node_id).map(|(id, _)| id));
        }

        subtree = subtree
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        for id in subtree {
            match &self.graph[id] {
                Node::Lambda { body, argument, .. } => {
                    writeln!(result, "{id} [label=\"{id}: λ{argument}\"]").unwrap();
                    writeln!(result, "{id} -> {body}").unwrap();
                }
                Node::Call {
                    function,
                    parameter,
                } => {
                    writeln!(result, "{id} [label=\"{id}: call\"]").unwrap();
                    writeln!(result, "{id} -> {function} [label=\"fn\"]").unwrap();
                    writeln!(result, "{id} -> {parameter} [label=\"param\"]").unwrap();

                    // Group function and parameter on same rank
                    writeln!(result, "{{ rank = same; {function}; {parameter}; }}").unwrap();
                    // Force horizontal order: function on the left, parameter on the right
                    writeln!(result, "{function} -> {parameter} [style=invis]").unwrap();
                }
                Node::Var { name, kind } => match kind {
                    VariableKind::Free => {
                        writeln!(result, "{id} [label=\"{id}: var {name}\"]").unwrap()
                    }
                    VariableKind::Bound { depth, .. } => {
                        writeln!(result, "{id} [label=\"{id}: var {name} ({depth}) \"]").unwrap()
                    }
                },
                Node::Token {
                    declaration,
                    variables,
                } => {
                    writeln!(result, "{id} [label=\"{id}: TOKEN {}\"]", declaration.name).unwrap();
                    for var in variables {
                        writeln!(result, "{id} -> {var}").unwrap();
                    }
                }
                Node::Data(value) => {
                    writeln!(result, "{id} [label=\"{id}: {:?}\"]", value).unwrap()
                }
                Node::Consumed(by) => {
                    writeln!(result, "{id} [label=\"{id}: consoomed by {by}\"]").unwrap();
                }
            }
        }

        writeln!(result, "}}").unwrap();
        result
    }

    pub fn is_debug_enabled(&self) -> bool {
        matches!(self.debug_config, DebugConfig::Enabled { .. })
    }

    pub fn add_debug_frame(&mut self, labels: Vec<(usize, &str)>) {
        if let DebugConfig::Enabled {
            auto_dump_every, ..
        } = self.debug_config
        {
            let dot = self.to_dot(labels);
            self.debug_frames.push(dot);

            if self.debug_frames.len() > self.debug_last_dump_at + auto_dump_every {
                self.dump_debug_frames();
                self.debug_last_dump_at = self.debug_frames.len();
            }
        }
    }

    pub fn enable_debug(&mut self, config: DebugConfig) {
        match &config {
            DebugConfig::Enabled { dump_path, .. } => {
                self.debug_config = config.clone();
                fs::remove_dir_all(dump_path).unwrap_or_default();
                fs::create_dir(dump_path).unwrap_or_default();
            }
            _ => unimplemented!(),
        }
    }

    pub fn dump_debug_frames(&self) {
        if let DebugConfig::Enabled { dump_path, .. } = &self.debug_config {
            println!("[DBG] Storing debug info into {dump_path}");

            for (id, frame) in self
                .debug_frames
                .iter()
                .enumerate()
                .skip(self.debug_last_dump_at)
            {
                let dot_filename = format!("{dump_path}/{:04}.dot", id);
                std::fs::write(dot_filename, frame).unwrap();
            }
        }
    }

    pub fn integrity_check(&self) {
        if self.is_debug_enabled() {
            self.debug_print_normalized();
            for id in self.traverse_subtree(self.root).map(|(id, _)| id) {
                if let Node::Lambda { argument, body } = &self.graph[id] {
                    for (expr, lambda_depth) in self.traverse_subtree(*body) {
                        match &self.graph[expr] {
                            Node::Var {
                                name,
                                kind: VariableKind::Bound { depth },
                            } if *depth == lambda_depth + 1 => {
                                if name != argument {
                                    println!(
                                        "{}",
                                        self.fmt_expr(self.unwrap_closure_chain(self.root))
                                    );
                                    println!(
                                        "Variable ({expr}) bound to lambda abstraction ({id}) "
                                    );
                                    dbg!(&self.graph[expr]);
                                    dbg!(&self.graph[id]);
                                    self.dump_debug_frames();
                                    panic!("Integrity check failed");
                                }
                            }
                            _ => {}
                        }
                    }
                };
            }
        }
    }
}
