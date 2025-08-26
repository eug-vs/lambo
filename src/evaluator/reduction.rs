use std::collections::HashSet;
use std::iter::from_fn;
use std::mem;

use crate::evaluator::{Graph, Node, VariableKind};

impl Graph {
    pub fn clone_subtree(&mut self, id: usize) -> usize {
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

    pub fn lift(&mut self, expr: usize) {
        let (outer_call, inner_call, lambda) = match self.graph[expr] {
            Node::Call {
                function: inner_call,
                ..
            } => match self.graph[inner_call] {
                Node::Call {
                    function: lambda, ..
                } => match self.graph[lambda] {
                    Node::Lambda { .. } => (expr, inner_call, lambda),
                    _ => return,
                },
                _ => return,
            },
            _ => return,
        };
        if self.is_debug_enabled() {
            self.add_debug_frame(vec![
                (expr, "performing lift"),
                (inner_call, "inner call"),
                (lambda, "lambda"),
            ]);
        }

        let mut buffer = mem::replace(
            &mut self.graph[outer_call],
            Node::Consumed("Lift".to_string()),
        );

        match &mut buffer {
            Node::Call { function, .. } => match &mut self.graph[lambda] {
                Node::Lambda { body, .. } => {
                    mem::swap(function, body);
                }
                _ => {
                    dbg!(&self.graph[lambda]);
                    unreachable!();
                }
            },
            _ => unreachable!(),
        }

        mem::swap(&mut buffer, &mut self.graph[inner_call]);
        mem::swap(&mut buffer, &mut self.graph[outer_call]);
        if self.is_debug_enabled() {
            self.add_debug_frame(vec![(expr, "after lift")]);
        }
    }
    pub fn assoc(&mut self, expr: usize) {
        let (outer_call, inner_call, lambda) = match self.graph[expr] {
            Node::Call {
                parameter: inner_call,
                ..
            } => match self.graph[inner_call] {
                Node::Call {
                    function: lambda, ..
                } => match self.graph[lambda] {
                    Node::Lambda { .. } => (expr, inner_call, lambda),

                    _ => return,
                },

                _ => return,
            },
            _ => return,
        };
        if self.is_debug_enabled() {
            self.add_debug_frame(vec![
                (expr, "performing assoc"),
                (inner_call, "inner call"),
                (lambda, "lambda"),
            ]);
        }

        let mut buffer = mem::replace(
            &mut self.graph[outer_call],
            Node::Consumed("Lift".to_string()),
        );

        match &mut buffer {
            Node::Call { parameter, .. } => match &mut self.graph[lambda] {
                Node::Lambda { body, .. } => {
                    mem::swap(parameter, body);
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }

        mem::swap(&mut buffer, &mut self.graph[inner_call]);
        mem::swap(&mut buffer, &mut self.graph[outer_call]);
        if self.is_debug_enabled() {
            self.add_debug_frame(vec![(expr, "after assoc")]);
        }
    }

    fn is_value(&self, expr: usize) -> bool {
        matches!(self.graph[expr], Node::Lambda { .. }) || (self.is_structure(expr))
    }

    /// Structure is a sequence of applications with a frozen var in its head.
    /// WARN: This implementation allows non-value parameters in the structure
    fn is_structure(&self, expr: usize) -> bool {
        match self.graph[expr] {
            Node::Call { function, .. } => self.is_structure(function),
            Node::Var {
                kind: VariableKind::Free,
                ..
            } => true,
            _ => false,
        }
    }

    /// Finds **locally free** variables in the subtree and
    /// adjusts their depth by some amount, usually after losing/gaining a binder
    fn adjust_depth(&mut self, expr: usize, by: usize) {
        let locally_free_variables = self
            .traverse_subtree(expr)
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
            .collect::<HashSet<_>>();

        for var_id in locally_free_variables {
            match &mut self.graph[var_id] {
                Node::Var {
                    kind: VariableKind::Bound { depth },
                    ..
                } => *depth += by,
                _ => unreachable!(),
            }
        }
    }

    /// Evaluates given expression to an ANSWER
    pub fn evaluate(&mut self, expr: usize, levels: &mut Vec<usize>) {
        let level = levels.len();
        if self.is_debug_enabled() {
            self.add_debug_frame(vec![(expr, "evaluate")]);
        }

        if let Node::Var {
            kind: VariableKind::Bound { depth },
            ..
        } = self.graph[expr]
        {
            let mut rest = levels.split_off(levels.len() - depth);
            let parameter = *rest.first().unwrap();
            if self.is_debug_enabled() {
                self.add_debug_frame(vec![(expr, "deref!"), (parameter, "with this")]);
            }
            self.evaluate(parameter, levels);
            debug_assert!(levels.len() == level, "Levels should have been cleaned up!");
            levels.append(&mut rest);

            let cloned_id = self.clone_subtree(parameter);
            self.adjust_depth(cloned_id, depth);
            self.graph.swap(expr, cloned_id);

            return;
        }

        if let Node::Call {
            function,
            parameter,
        } = self.graph[expr]
        {
            self.evaluate(function, levels);

            if self.is_value(function) {
                if let Node::Lambda { body, .. } = self.graph[function] {
                    levels.push(parameter);
                    self.evaluate(body, levels);
                    levels.truncate(level);
                }
            } else {
                self.lift(expr);
                self.adjust_depth(parameter, 1); // Lift will add 1 binder
                levels.truncate(level);
                self.evaluate(expr, levels)
            }
        }
    }

    pub fn unwrap_closure_chain(&mut self, expr: usize, mut context: Vec<usize>) -> usize {
        if let Node::Call {
            function,
            parameter,
        } = self.graph[expr]
        {
            if let Node::Lambda { body, .. } = &self.graph[function] {
                context.push(parameter);
                return self.unwrap_closure_chain(*body, context);
            }
        };

        expr
    }
}
