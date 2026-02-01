

use crate::ast::{Node, AST};

impl AST {
    #[tracing::instrument(skip(self))]
    pub fn garbage_collect(&mut self) {
        loop {
            let unsued_closures = self
                .graph
                .node_indices()
                .filter(|&node_id| {
                    matches!(
                        self.graph.node_weight(node_id).unwrap(),
                        Node::Closure { .. }
                    ) && self.binder_references(node_id).next().is_none()
                })
                .collect::<Vec<_>>();

            if unsued_closures.is_empty() {
                break;
            }
            for closure_id in unsued_closures {
                let parameter = self.remove_closure(closure_id).unwrap();
                self.remove_subtree(parameter);
            }
        }
    }
}
