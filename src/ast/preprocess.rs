

use crate::ast::{Node, AST};

impl AST {
    #[tracing::instrument(skip(self))]
    pub fn garbage_collect(&mut self) -> usize {
        let unused_closures = self
            .graph
            .node_indices()
            .filter(|&node_id| {
                matches!(
                    self.graph.node_weight(node_id).unwrap(),
                    Node::Closure { .. }
                ) && self.binder_references(node_id).next().is_none()
            })
            .collect::<Vec<_>>();

        let count = unused_closures.len();

        for closure_id in unused_closures {
            let parameter = self.remove_closure(closure_id).unwrap();
            self.remove_subtree(parameter);
        }

        count
    }
}
