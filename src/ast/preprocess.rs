use std::collections::HashSet;

use petgraph::{Direction, graph::NodeIndex, visit::EdgeRef};

use crate::ast::{AST, Edge, Node, VariableKind};

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

    #[tracing::instrument(skip(self))]
    pub fn lift_mfes(&mut self) {
        for id in self.graph.node_indices().collect::<Vec<_>>() {
            if let Node::Lambda { .. } = self.graph.node_weight(id).unwrap() {
                // Only lift MFEs from lambdas in parameter position
                if self
                    .graph
                    .edges_directed(id, Direction::Incoming)
                    .filter(|e| matches!(e.weight(), Edge::Parameter))
                    .next()
                    .is_some()
                {
                    let (mfes, _) = self.find_mfes(id, &mut HashSet::new());
                    let mut highest = id;
                    for mfe in mfes {
                        self.add_debug_frame_with_annotation(mfe, "MFE");

                        let mfe_var = self.graph.add_node(Node::Variable(VariableKind::Bound));
                        self.migrate_node(mfe, mfe_var);
                        // Create new MFE closure
                        let mfe_closure = self.graph.add_node(Node::Closure {
                            argument_name: String::from("mfe").into(),
                        });
                        self.graph.add_edge(mfe_var, mfe_closure, Edge::Binder(0));
                        self.graph.add_edge(mfe_closure, mfe, Edge::Parameter);

                        // Add this closure on top of current parameter chain
                        self.migrate_node(highest, mfe_closure);
                        self.graph.add_edge(mfe_closure, highest, Edge::Body);
                        highest = mfe_closure;
                    }
                }
            }
        }
    }

    /// Returns (list of MFEs, list of variables)
    fn find_mfes(
        &self,
        node_id: NodeIndex,
        blacklist: &HashSet<NodeIndex>,
    ) -> (HashSet<NodeIndex>, HashSet<NodeIndex>) {
        let current_node = self.graph.node_weight(node_id).unwrap();

        let local_blacklist = match current_node {
            Node::Variable(VariableKind::Bound) | Node::Data { .. } => {
                return (
                    [].into(),
                    [node_id].into(), // Single variable
                );
            }
            Node::Lambda { .. } | Node::Closure { .. } => {
                let mut b = blacklist.clone();
                for edge in self
                    .graph
                    .edges_directed(node_id, Direction::Incoming)
                    .filter(|e| matches!(e.weight(), Edge::Binder(_)))
                {
                    b.insert(edge.source());
                }
                Some(b)
            }
            _ => None,
        };

        let blacklist = match &local_blacklist {
            Some(b) => b,
            None => blacklist,
        };

        let children_result = self
            .graph
            .neighbors(node_id)
            .map(|child| self.find_mfes(child, blacklist))
            .reduce(|(a_mfes, a_vars), (b_mfes, b_vars)| {
                (
                    a_mfes.union(&b_mfes).copied().collect(),
                    a_vars.union(&b_vars).copied().collect(),
                )
            })
            .unwrap_or_default();

        if let Node::Application = current_node {
            if children_result
                .1
                .iter()
                .all(|var| blacklist.get(var).is_none())
            {
                return ([node_id].into(), children_result.1);
            }
        }

        children_result
    }
}
