use std::{collections::HashSet, fs};

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
        writeln!(result, "ROOT -> {}", self.root).unwrap();

        let mut subtree = self.subtree_under(self.root);
        for (label_id, (node_id, name)) in labels.iter().enumerate() {
            writeln!(result, "LABEL{label_id} [label=\"{name}\", color=\"red\"]",).unwrap();
            writeln!(result, "LABEL{label_id} -> {}", node_id).unwrap();
            subtree.append(&mut self.subtree_under(*node_id));
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
                fs::remove_dir(dump_path).unwrap_or_default();
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

    fn subtree_under(&self, node: usize) -> Vec<usize> {
        self.traverse_subtree(node).map(|(id, _)| id).collect()
    }
}
