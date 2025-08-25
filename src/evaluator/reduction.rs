use std::iter::{from_fn, once};
use std::mem;

use crate::{
    evaluator::{EvaluationOrder, Graph, Node},
    VariableKind,
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

    /// WARN: since this function borrows &self, you need to collect the output first
    /// Filtering the output before collect (rather then after) will improve performance
    pub fn traverse_subtree(&self, root: usize) -> impl Iterator<Item = (usize, usize)> {
        let mut stack = vec![(root, 0)];

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
                _ => {}
            };
            Some((node, lambda_depth_to_root))
        })
    }

    fn locally_free_variables(&self, expr: usize) -> Vec<usize> {
        self.traverse_subtree(expr)
            .filter(|&(id, lambdas_gained_from_root)| {
                matches!(
                    self.graph[id],
                    Node::Var {
                        kind: VariableKind::Bound { depth },
                        ..
                    } if depth > lambdas_gained_from_root
                )
            })
            .map(|(id, _)| id)
            .collect::<Vec<_>>()
    }

    /// Finds **locally free** variables in the subtree and
    /// adjusts their depth by some amount, usually after losing/gaining a binder
    fn adjust_depth(&mut self, expr: usize, by: isize) {
        let locally_free_variables = self.locally_free_variables(expr);

        if self.debug {
            let msg = format!("adjusting depth here by {by}");
            self.add_debug_frame(if locally_free_variables.is_empty() {
                vec![(expr, "No need to adjust depth")]
            } else {
                locally_free_variables
                    .iter()
                    .map(|&id| (id, msg.as_str()))
                    .collect()
            });
        }

        for var_id in locally_free_variables {
            match &mut self.graph[var_id] {
                Node::Var {
                    kind: VariableKind::Bound { depth },
                    ..
                } => *depth = (*depth as isize + by) as usize,
                _ => unreachable!(),
            }
        }
    }

    fn substitute(&mut self, expr: usize, with: usize) {
        let substitution_targets = self
            .traverse_subtree(expr)
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
            .collect::<Vec<_>>();

        if self.debug {
            self.add_debug_frame(if substitution_targets.is_empty() {
                vec![(expr, "no substitution targets in this subtree")]
            } else {
                substitution_targets
                    .iter()
                    .map(|&(id, _)| (id, "replacing"))
                    .chain(once((with, "with this")))
                    .collect()
            });
        }

        for (index, &(var_id, lambdas_gained_from_root)) in substitution_targets.iter().enumerate()
        {
            let new_id = if index < substitution_targets.len() - 1 {
                self.clone_subtree(with)
            } else {
                with // No need to clone the last target
            };
            self.graph[var_id] = mem::replace(
                &mut self.graph[new_id],
                Node::Consumed("In substitution".to_string()),
            );
            self.adjust_depth(var_id, (lambdas_gained_from_root + 1) as isize);
        }
    }

    pub fn evaluate(&mut self, expr: usize, order: EvaluationOrder) {
        if self.debug {
            self.add_debug_frame(vec![(expr, "evaluate")]);
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
                    self.add_debug_frame(vec![(function, "after function resolve")]);
                }

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
                        self.adjust_depth(expr, -1);
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
                        self.add_debug_frame(vec![(expr, "after lambda eval")]);
                    }
                }
                _ => {}
            },
            Node::Consumed(_) => self.panic_consumed_node(expr),
        }
    }
}
