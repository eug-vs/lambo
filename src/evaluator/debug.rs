use std::{collections::HashSet, fs};

use crate::{
    evaluator::{Node, VariableKind},
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

    pub fn add_debug_frame(&mut self, labels: Vec<(usize, &str)>) {
        if self.debug {
            let dot = self.to_dot(labels);
            self.debug_frames.push(dot);
        }
    }

    pub fn dump_debug_frames(&self, dir: &str) {
        if self.debug {
            println!("[DBG] Storing debug info into {dir}");
            fs::remove_dir(dir).unwrap_or_default();
            fs::create_dir(dir).unwrap_or_default();
            for (id, frame) in self.debug_frames.iter().enumerate() {
                let dot_filename = format!("{dir}/{:03}.dot", id);
                std::fs::write(dot_filename, frame).unwrap();
            }
        }
    }

    fn subtree_under(&self, node: usize) -> Vec<usize> {
        self.traverse_subtree(node).map(|(id, _)| id).collect()
    }
}
