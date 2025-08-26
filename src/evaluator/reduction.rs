use std::collections::HashSet;
use std::iter::from_fn;
use std::mem;

use crate::evaluator::{Graph, Node, VariableKind};

pub enum StepResult {
    Answer,
    Needed(usize, usize),
}

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
    pub fn evaluate(&mut self, expr: usize, level: usize) -> StepResult {
        self.add_debug_frame(vec![(expr, "evaluate")]);
        let mut loops = 0;

        if let Node::Var {
            kind: VariableKind::Bound { depth },
            ..
        } = &self.graph[expr]
        {
            return StepResult::Needed(expr, level - depth);
        }

        while !self.is_answer(expr) {
            loops += 1;
            if loops > 100 {
                self.panic_consumed_node(expr);
            }
            self.add_debug_frame(vec![(expr, "not an answer, looping")]);
            if let Node::Call {
                function,
                parameter,
            } = self.graph[expr]
            {
                if let StepResult::Needed(needed_id, needed_level) = self.evaluate(function, level)
                {
                    if needed_level < level {
                        return StepResult::Needed(needed_id, needed_level);
                    }
                };

                if self.is_value(function) {
                    if let Node::Lambda { body, .. } = self.graph[function] {
                        match self.evaluate(body, level + 1) {
                            StepResult::Needed(needed_id, needed_level) => {
                                self.add_debug_frame(vec![
                                    (needed_id, "needed variable"),
                                    (body, "in this body"),
                                ]);
                                if needed_level < level {
                                    return StepResult::Needed(needed_id, needed_level);
                                }
                                if needed_level > level {
                                    unreachable!();
                                }
                                // Function body has a hole in it!
                                // Preparing deref or assoc, both need answer on parameter position
                                if let StepResult::Needed(needed_id, needed_level) =
                                    self.evaluate(parameter, level)
                                {
                                    return StepResult::Needed(needed_id, needed_level);
                                };
                                // Parameter is an answer at this point!
                                if self.is_value(parameter) {
                                    let cloned_id = self.clone_subtree(parameter);
                                    let depth_adjustment = match self.graph[needed_id] {
                                        Node::Var {
                                            kind: VariableKind::Bound { depth },
                                            ..
                                        } => depth,
                                        _ => unreachable!(),
                                    };
                                    self.adjust_depth(cloned_id, depth_adjustment);
                                    self.graph.swap(needed_id, cloned_id);

                                    self.add_debug_frame(vec![
                                        (expr, "deref!"),
                                        (needed_id, "substituted here"),
                                        (parameter, "from this"),
                                    ]);
                                } else {
                                    self.add_debug_frame(vec![(expr, "assoc!")]);
                                    self.adjust_depth(function, 1); // Assoc will add 1 binder
                                    self.assoc(expr);
                                }
                            }
                            StepResult::Answer => {
                                // Nothing to substitute!?
                                return StepResult::Answer;
                            }
                        };
                    }
                } else {
                    self.add_debug_frame(vec![(expr, "function is answer but not value: lift!")]);
                    self.lift(expr);
                    self.adjust_depth(parameter, 1); // Lift will add 1 binder
                }
            }
        }
        StepResult::Answer
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
