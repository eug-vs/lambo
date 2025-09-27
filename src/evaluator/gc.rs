use std::collections::HashSet;

use crate::evaluator::{Graph, Node, NodeID, VariableKind};

impl Graph {
    pub fn try_garbage_collect(&mut self, expr: NodeID) -> bool {
        match &self.graph[expr.get()] {
            Node::Closure {
                body,
                parent_environment,
                ..
            } if !self
                .traverse_subtree(body.get())
                .any(|(id, lambdas_gained_from_root)| {
                    matches!(
                        self.graph[id],
                        Node::Var {
                        kind: VariableKind::Bound { depth },
                        ..
                    } if depth == lambdas_gained_from_root + 1
                    )
                }) =>
            {
                let body = body.get();
                let parent_environment = parent_environment.get();

                self.add_debug_frame(vec![(expr.get(), "garbage collect")]);

                self.adjust_depth(body, -1);

                let subtree = self.traverse_subtree(body).collect::<HashSet<_>>();
                for (id, _) in subtree {
                    match &mut self.graph[id] {
                        Node::Closure {
                            parent_environment: parent,
                            ..
                        } if parent.get() == expr.get() => parent.set(parent_environment),
                        _ => {}
                    }
                }

                expr.set(body);
                true
            }
            _ => false,
        }
    }

    fn garbage_collect(&mut self, expr: NodeID, count: &mut usize) {
        if self.try_garbage_collect(expr.clone()) {
            *count += 1;
        }

        match &self.graph[expr.get()] {
            Node::Call {
                function,
                parameter,
            } => {
                let function = function.clone();
                let parameter = parameter.clone();

                self.garbage_collect(function, count);
                self.garbage_collect(parameter, count);
            }
            Node::Lambda { body, .. } => {
                self.garbage_collect(body.clone(), count);
            }
            Node::Closure {
                body, parameter, ..
            } => {
                let body = body.clone();
                let parameter = parameter.clone();
                self.garbage_collect(body, count);
                self.garbage_collect(parameter, count);
            }
            _ => {}
        };
    }

    pub fn gc_root(&mut self) {
        let mut total = 0;
        let mut passes = 0;
        loop {
            passes += 1;
            let mut cleaned_up_nodes = 0;
            self.garbage_collect(self.root.clone(), &mut cleaned_up_nodes);
            if cleaned_up_nodes > 0 {
                total += cleaned_up_nodes;
            } else {
                break;
            };
        }
        println!("[GC] cleaned up {total} nodes in total ({passes} passes)");
        self.add_debug_frame(vec![(
            self.root.get(),
            format!("after GC ({total} nodes in {passes} passes)").as_str(),
        )]);
    }
}
