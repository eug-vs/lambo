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
    pub fn adjust_depth(&mut self, expr: usize, by: usize) {
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

    /// Finds the sequence of closure nodes that contains target expression
    pub fn find_closure_path(&self, expr: usize, target: usize, path: &mut Vec<usize>) -> bool {
        if expr == target {
            return true;
        }
        match &self.graph[expr] {
            Node::Lambda { body, .. } => {
                if self.find_closure_path(*body, target, path) {
                    return true;
                }
            }
            Node::Call {
                function,
                parameter,
            } => {
                let is_closure = matches!(self.graph[*function], Node::Lambda { .. });
                if is_closure {
                    path.push(expr);
                }
                if self.find_closure_path(*function, target, path) {
                    return true;
                }
                if is_closure {
                    path.pop();
                }
                if self.find_closure_path(*parameter, target, path) {
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    fn find_binding_closure(&mut self, expr: usize) -> usize {
        let mut levels = Vec::new();
        self.find_closure_path(self.root, expr, &mut levels);

        let depth = match self.graph[expr] {
            Node::Var {
                kind: VariableKind::Bound { depth },
                ..
            } => depth,
            _ => unreachable!(),
        };

        levels[levels.len() - depth]
    }

    /// Evaluates given expression to an ANSWER
    pub fn evaluate(&mut self, expr: usize) {
        if self.is_debug_enabled() {
            self.add_debug_frame(vec![(expr, "eval")]);
            self.integrity_check();
        }

        match self.graph[expr] {
            Node::Call {
                function,
                parameter,
            } => {
                self.evaluate(function);

                if self.is_value(function) {
                    if let Node::Lambda { body, .. } = self.graph[function] {
                        self.evaluate(body);
                    }
                } else {
                    // Closure on function position: LIFT
                    if self.is_debug_enabled() {
                        self.add_debug_frame(vec![(expr, "lift")]);
                    }
                    self.lift(expr);
                    self.adjust_depth(parameter, 1); // Lift will add 1 binder
                                                     // Restart evaluation of this node
                    self.evaluate(expr)
                }
            }
            Node::Var {
                kind: VariableKind::Bound { depth },
                ..
            } => {
                let binding_closure_id = self.find_binding_closure(expr);

                match self.graph[binding_closure_id] {
                    Node::Call {
                        parameter,
                        function,
                    } => {
                        // Compute an answer on parameter position of a binding closure
                        self.evaluate(parameter);

                        if self.is_value(parameter) {
                            // Parameter is a value now, time to deref!
                            if self.is_debug_enabled() {
                                self.add_debug_frame(vec![
                                    (expr, "deref"),
                                    (parameter, "with this"),
                                ]);
                            }

                            let cloned_id = self.clone_subtree(parameter);
                            self.adjust_depth(cloned_id, depth);
                            self.graph.swap(expr, cloned_id);
                        } else {
                            // Apply assoc if we have closure on parameter position
                            if self.is_debug_enabled() {
                                self.add_debug_frame(vec![
                                    (expr, "hole in context"),
                                    (parameter, "parameter"),
                                    (binding_closure_id, "assoc"),
                                ]);
                            }
                            self.adjust_depth(function, 1); // Assoc will add 1 binder
                            self.assoc(binding_closure_id);

                            // Restart evaluation of this node
                            self.evaluate(expr)
                        }
                    }
                    _ => unreachable!("Closure must be a call node"),
                }
            }
            _ => {} // Everything else is already a value
        }
    }

    pub fn unwrap_closure_chain(&self, expr: usize) -> usize {
        if let Node::Call { function, .. } = self.graph[expr] {
            if let Node::Lambda { body, .. } = &self.graph[function] {
                return self.unwrap_closure_chain(*body);
            }
        };

        expr
    }
}
