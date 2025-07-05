use std::mem;

use crate::{
    VariableKind,
    evaluator::{EvaluationOrder, Graph, Node},
};

impl Graph {
    fn clone_subtree(&mut self, id: usize) -> usize {
        let mut node = self.graph[id].clone();
        match &mut node {
            Node::Var { .. } => {}
            Node::Lambda { body, .. } => *body = self.clone_subtree(*body),
            Node::Call {
                function,
                parameter,
                ..
            } => {
                *function = self.clone_subtree(*function);
                *parameter = self.clone_subtree(*parameter);
            }
            _ => unreachable!("{:?}", node),
        };
        self.graph.push(node);
        self.graph.len() - 1
    }

    fn adjust_depth(&mut self, expr: usize, cutoff: usize, by: isize) {
        if self.debug {
            self.debug_frames.push(self.to_dot(vec![(
                expr,
                format!("adjusting depth here by {by} (only if >= {cutoff})").as_str(),
            )]));
            // TMP:
            // fs::remove_dir_all("./debug/infinite-loop").unwrap();
            // self.store_debug_info("./debug/infinite-loop");
        }
        // Mutable branch
        match &mut self.graph[expr] {
            Node::Var {
                kind: VariableKind::Bound { depth, .. },
                ..
            } if *depth >= cutoff => {
                *depth = (*depth as isize + by) as usize;
            }
            _ => {}
        }
        // Immutable branch - pass &mut self down to recursive call
        match self.graph[expr] {
            Node::Lambda { body, .. } => self.adjust_depth(body, cutoff + 1, by),
            Node::Call {
                function,
                parameter,
            } => {
                self.adjust_depth(function, cutoff, by);
                self.adjust_depth(parameter, cutoff, by);
            }
            Node::Consumed(_) => self.panic_consumed_node(expr),
            _ => {} // Noop for vars and thunks
        }
    }

    fn substitute(&mut self, expr: usize, with: usize, at_depth: usize) {
        if self.debug {
            self.debug_frames.push(self.to_dot(vec![
                (expr, format!("replacing here at depth {at_depth}").as_str()),
                (with, "with this"),
            ]));
        }
        match self.graph[expr] {
            Node::Lambda { body, .. } => self.substitute(body, with, at_depth + 1),
            Node::Call {
                function,
                parameter,
            } => {
                self.substitute(function, with, at_depth);
                self.substitute(parameter, with, at_depth);
            }
            Node::Var {
                kind: VariableKind::Bound { depth, .. },
                ..
            } if depth == at_depth => {
                let new_id = self.clone_subtree(with);
                self.graph[expr] = mem::replace(
                    &mut self.graph[new_id],
                    Node::Consumed("In substitution".to_string()),
                );
                let lambdas_gained = at_depth; // We rely that root call to substitute always has at_depth 1
                if lambdas_gained > 0 {
                    self.adjust_depth(expr, 1, lambdas_gained as isize);
                }
            }
            _ => {} // Do not substitute in other vars
        }
    }

    pub fn evaluate(&mut self, expr: usize, order: EvaluationOrder) {
        if self.debug {
            self.debug_frames
                .push(self.to_dot(vec![(expr, "evaluate")]));
        }
        match self.graph[expr] {
            Node::Var { .. } => {}
            Node::Call {
                function,
                parameter,
            } => {
                // Convert functon to WNHF first
                // WARN: here we actually compute NF
                self.evaluate(function, order);
                if self.debug {
                    self.debug_frames
                        .push(self.to_dot(vec![(function, "after function resolve")]));
                }
                if self.handle_builtins(expr) {
                    return;
                };

                match self.graph[function] {
                    Node::Var { .. } => self.evaluate(parameter, order),
                    Node::Call { .. } => {} // Call already in WHNF, not much to do
                    Node::Lambda { body, .. } => {
                        if self.debug {
                            self.add_debug_frame(vec![
                                (expr, "resolving this call"),
                                (function, "this defines variable"),
                                (parameter, "moving this"),
                                (body, "into this subtree"),
                            ]);
                        }
                        self.graph[expr] = mem::replace(
                            &mut self.graph[body],
                            Node::Consumed("evaluate_lambda_body".to_string()),
                        );
                        self.substitute(expr, parameter, 1);
                        self.adjust_depth(expr, 1, -1);
                        if self.debug {
                            self.add_debug_frame(vec![(expr, "after substitute")]);
                        }

                        self.evaluate(expr, order);
                    }
                    Node::Consumed(_) => self.panic_consumed_node(function),
                }
            }
            Node::Lambda { body, .. } => match order {
                EvaluationOrder::Normal => {
                    self.evaluate(body, order);
                    if self.debug {
                        self.debug_frames
                            .push(self.to_dot(vec![(expr, "after lambda eval")]));
                    }
                }
                _ => {}
            },
            Node::Consumed(_) => self.panic_consumed_node(expr),
        }
    }
}
