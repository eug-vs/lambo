use std::collections::HashSet;
use std::iter::{from_fn, once};
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
            Node::Thunk { node, .. } => *node = self.clone_subtree(*node),
            _ => unreachable!("{:?}", node),
        };
        self.graph.push(node);
        self.graph.len() - 1
    }

    /// WARN: since this function borrows &self, you need to collect the output first
    /// Filtering the output before collect (rather then after) will improve performance
    pub fn traverse_subtree(
        &self,
        root: usize,
        visit_thunks: bool,
    ) -> impl Iterator<Item = (usize, usize)> {
        let mut stack = vec![(root, 0 as usize)];

        from_fn(move || {
            let (node, lambda_depth_to_root) = stack.pop()?;
            match self.graph[node] {
                Node::Lambda { body, .. } => {
                    stack.push((body, lambda_depth_to_root + 1));
                }
                Node::Call {
                    function,
                    parameter,
                } => {
                    stack.push((function, lambda_depth_to_root));
                    stack.push((parameter, lambda_depth_to_root));
                }
                Node::Thunk {
                    node,
                    depth_adjustment,
                } => {
                    if visit_thunks {
                        stack.push((
                            node,
                            ((lambda_depth_to_root as isize) - depth_adjustment)
                                .try_into()
                                .unwrap_or_default(),
                        ));
                    }
                }
                _ => {}
            };
            Some((node, lambda_depth_to_root))
        })
    }

    /// Finds **locally free** variables in the subtree and
    /// adjusts their depth by some amount, usually after losing/gaining a binder
    fn adjust_for_lost_binder(&mut self, expr: usize) {
        let traversed = self.traverse_subtree(expr, false).collect::<HashSet<_>>();

        // if self.debug {
        //     let msg = format!("adjusting depth here");
        //     self.add_debug_frame(if locally_free_variables.is_empty() {
        //         vec![(expr, "No need to adjust depth")]
        //     } else {
        //         locally_free_variables
        //             .iter()
        //             .map(|&id| (id, msg.as_str()))
        //             .collect()
        //     });
        // }

        for (var_id, lambda_depth_to_root) in traversed {
            match &mut self.graph[var_id] {
                Node::Var {
                    kind: VariableKind::Bound { depth },
                    ..
                } if *depth > lambda_depth_to_root => *depth -= 1,
                Node::Thunk {
                    depth_adjustment, ..
                } => {
                    if *depth_adjustment > 0 {
                        *depth_adjustment -= 1;
                    }
                    if *depth_adjustment < 0 {
                        self.add_debug_frame(vec![(var_id, "depth adjustment wrong")]);
                        self.panic_consumed_node(var_id);
                    }
                    assert!(
                        *depth_adjustment >= 0,
                        "Depth adjustment can't be less then 0"
                    );
                }
                _ => {}
            }
        }
    }

    fn substitute(&mut self, lambda_body: usize, with: usize) {
        let substitution_targets = self
            .traverse_subtree(lambda_body, true)
            .filter(|&(id, lambdas_gained_from_root)| {
                matches!(
                    self.graph[id],
                    Node::Var {
                        kind: VariableKind::Bound { depth },
                        ..
                    } if depth
                        .checked_sub(lambdas_gained_from_root)
                        .unwrap_or_default()
                        == 1
                )
            })
            .collect::<HashSet<_>>();

        if self.debug {
            self.add_debug_frame(if substitution_targets.is_empty() {
                vec![(lambda_body, "no substitution targets in this subtree")]
            } else {
                substitution_targets
                    .iter()
                    .map(|&(id, _)| (id, "replacing"))
                    .chain(once((with, "with this")))
                    .collect()
            });
        }

        for (var_id, lambdas_gained_from_root) in substitution_targets {
            let mut thunk = Node::Thunk {
                node: with,
                depth_adjustment: (lambdas_gained_from_root as isize) + 1, // TODO: seems to work without + 1
            };
            self.graph[var_id] =
                mem::replace(&mut thunk, Node::Consumed("In substitution".to_string()));
        }
    }

    pub fn jump_through_thunks(&self, expr: usize) -> usize {
        match self.graph[expr] {
            Node::Thunk { node, .. } => self.jump_through_thunks(node),
            _ => expr,
        }
    }

    pub fn evaluate(&mut self, expr: usize, order: EvaluationOrder) {
        if self.debug {
            self.add_debug_frame(vec![(expr, "evaluate")]);
        }
        match self.graph[expr] {
            Node::Var { .. } => {}
            Node::Call {
                mut function,
                parameter,
            } => {
                // Convert functon to WNHF first
                // WARN: here we actually compute NF
                self.evaluate(function, order);
                if self.debug {
                    self.add_debug_frame(vec![(function, "after function resolve")]);
                }
                if self.handle_builtins(expr) {
                    return;
                };

                {
                    let subtree_under_thunks = self.jump_through_thunks(function);
                    if subtree_under_thunks != function {
                        function = self.clone_subtree(subtree_under_thunks);
                    }
                }

                self.add_debug_frame(vec![(function, "after resolve thunks")]);

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
                        self.substitute(expr, parameter);
                        self.adjust_for_lost_binder(expr);
                        if self.debug {
                            self.add_debug_frame(vec![(expr, "after substitute")]);
                        }

                        self.evaluate(expr, order);
                    }
                    Node::Consumed(_) => self.panic_consumed_node(function),
                    Node::Thunk { .. } => self.panic_consumed_node(function),
                }
            }
            Node::Lambda { body, .. } => match order {
                EvaluationOrder::Normal => {
                    self.evaluate(body, order);
                    if self.debug {
                        self.add_debug_frame(vec![(expr, "after lambda eval")]);
                    }
                }
                _ => {}
            },
            Node::Consumed(_) => self.panic_consumed_node(expr),
            Node::Thunk { node, .. } => self.evaluate(node, order),
        }
    }
}
