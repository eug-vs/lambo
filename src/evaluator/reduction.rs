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
        if self.debug {
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
        if self.debug {
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
        if self.debug {
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
        if self.debug {
            self.add_debug_frame(vec![(expr, "after assoc")]);
        }
    }

    fn is_value(&self, expr: usize) -> bool {
        matches!(self.graph[expr], Node::Lambda { .. }) || (self.is_structure(expr))
    }

    fn is_answer(&self, expr: usize) -> bool {
        self.is_value(expr)
            || match self.graph[expr] {
                Node::Call { function, .. } => match &self.graph[function] {
                    Node::Lambda { body, .. } => {
                        return self.is_answer(*body);
                    }
                    _ => false,
                },
                _ => false,
            }
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

    fn is_bound_variable(&self, lambda_body: usize, var: usize) -> Option<usize> {
        let (_, lambda_depth_to_root) = self
            .traverse_subtree(lambda_body)
            .find(|&(id, _)| id == var)
            .unwrap();

        match self.graph[var] {
            Node::Var {
                kind: VariableKind::Bound { depth },
                ..
            } if depth == lambda_depth_to_root + 1 => Some(depth),
            _ => None,
        }
    }

    /// Implements evaluation context definition.
    /// Returns an ID of the "hole" in context E,
    /// a.k.a the term currently "blocking" evaluation of a term
    fn get_needed_id(&mut self, expr: usize) -> usize {
        match self.graph[expr] {
            Node::Call {
                function,
                parameter,
            } => match self.graph[function] {
                Node::Lambda { body, .. } => {
                    let body_redex = self.get_needed_id(body);

                    if self.is_bound_variable(body, body_redex).is_some() {
                        self.get_needed_id(parameter)
                    } else {
                        body_redex
                    }
                }
                _ => self.get_needed_id(function),
            },
            _ => expr,
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

    /// Step is considered applying one of the axioms (deref, assoc, lift) on the current expr
    fn evaluation_step(&mut self, expr: usize) {
        self.add_debug_frame(vec![(expr, format!("evaluation step {expr}").as_str())]);

        if self.is_answer(expr) {
            self.add_debug_frame(vec![(expr, "already an answer")]);
            return;
        }

        if let Node::Call {
            function,
            parameter,
        } = self.graph[expr]
        {
            // All axioms demand an answer on function position
            while !self.is_answer(function) {
                return self.evaluation_step(function);
            }

            if self.is_value(function) {
                if let Node::Lambda { body, .. } = self.graph[function] {
                    let body_redex = self.get_needed_id(body);
                    self.add_debug_frame(vec![(body_redex, "redex"), (body, "in this body")]);

                    if let Some(depth) = self.is_bound_variable(body, body_redex) {
                        // Both deref and assoc demand answer on argument position
                        while !self.is_answer(parameter) {
                            return self.evaluation_step(parameter);
                        }

                        if self.is_value(parameter) {
                            let cloned_id = self.clone_subtree(parameter);
                            self.adjust_depth(cloned_id, depth);
                            self.graph.swap(body_redex, cloned_id);

                            self.add_debug_frame(vec![
                                (expr, "deref!"),
                                (body_redex, "substituted here"),
                                (parameter, "from this"),
                            ]);
                        } else {
                            self.add_debug_frame(vec![(expr, "assoc!")]);
                            self.adjust_depth(function, 1); // Assoc will add 1 binder
                            self.assoc(expr);
                        }
                    } else {
                        self.add_debug_frame(vec![
                            (
                                expr,
                                "Closure but not an answer. Can't apply axioms at this node",
                            ),
                            (body, "Advance evaluation into the body"),
                        ]);
                        self.evaluation_step(body)
                    }
                }
            } else {
                self.add_debug_frame(vec![(expr, "function is answer but not value: lift!")]);
                self.lift(expr);
                self.adjust_depth(parameter, 1); // Lift will add 1 binder
            }
        }
    }

    pub fn evaluate(&mut self, expr: usize) {
        while !self.is_answer(expr) {
            self.evaluation_step(expr);
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
