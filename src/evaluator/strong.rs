use std::collections::HashSet;

use crate::evaluator::{Graph, Node, VariableKind};

impl Graph {
    pub fn debug_print_normalized(&self) {
        let mut copy = self.clone();
        copy.evaluate_strong(copy.root);
        println!("Strong result: {}", copy);
    }
    fn evaluate_strong(&mut self, expr: usize) {
        if let Node::Call {
            function,
            parameter,
        } = self.graph[expr]
        {
            self.evaluate_strong(function);

            if let Node::Lambda { body, .. } = self.graph[function] {
                let substitution_targets = self
                    .traverse_subtree(body)
                    .filter(|&(id, lambdas_gained_from_root)| {
                        matches!(
                            self.graph[id],
                            Node::Var {
                                kind: VariableKind::Bound { depth },
                                ..
                            } if depth == lambdas_gained_from_root + 1
                        )
                    })
                    .collect::<HashSet<_>>();

                for (idx, &(target_id, depth)) in substitution_targets.iter().enumerate() {
                    let cloned_id = if idx == substitution_targets.len() - 1 {
                        parameter
                    } else {
                        self.clone_subtree(parameter)
                    };
                    self.adjust_depth(cloned_id, depth);
                    self.graph.swap(target_id, cloned_id);
                }

                self.evaluate_strong(body);
                self.graph.swap(body, expr);
            }
        }
    }
}
