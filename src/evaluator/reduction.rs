use std::collections::HashSet;
use std::iter::from_fn;
use std::mem;

use crate::evaluator::{Graph, Node, VariableKind};

#[derive(Debug)]
pub struct ClosurePath(pub Vec<usize>);

impl ClosurePath {
    fn new() -> Self {
        ClosurePath(Vec::new())
    }
    fn get_at_depth(&self, depth: usize) -> usize {
        self.0[self.0.len() - depth]
    }

    fn register(&mut self, closure_id: usize) {
        self.0.push(closure_id);
    }
    fn register_after_depth(&mut self, closure_id: usize, depth: usize) {
        self.0.insert(self.0.len() + 1 - depth, closure_id);
    }

    fn backtrack_before_closure(&mut self, closure_id: usize) -> Vec<usize> {
        let index = self.0.iter().rposition(|id| *id == closure_id).unwrap();

        self.0.split_off(index)
    }
    fn restore_backtrack(&mut self, mut rest: Vec<usize>) {
        self.0.append(&mut rest);
    }
}

impl Graph {
    pub fn clone_subtree(&mut self, id: usize) -> usize {
        let mut node = self.graph[id].clone();
        match &mut node {
            Node::Var { .. } | Node::Primitive { .. } => {}
            Node::Lambda { body, .. } => *body = self.clone_subtree(*body),
            Node::Call {
                function,
                parameter,
                ..
            } => {
                *function = self.clone_subtree(*function);
                *parameter = self.clone_subtree(*parameter);
            }
            Node::Data {
                constructor_params, ..
            } => {
                for child in constructor_params {
                    *child = self.clone_subtree(*child);
                }
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
            match &self.graph[node] {
                Node::Lambda { body, .. } => {
                    stack.push((*body, lambda_depth_to_root + 1));
                }
                Node::Call {
                    function,
                    parameter,
                } => {
                    stack.push((*function, lambda_depth_to_root));
                    stack.push((*parameter, lambda_depth_to_root));
                }
                Node::Data {
                    constructor_params, ..
                } => {
                    for child in constructor_params {
                        stack.push((*child, lambda_depth_to_root));
                    }
                }
                _ => {}
            };
            Some((node, lambda_depth_to_root))
        })
    }

    /// Perform *lift* inference rule and returns ID of new closure
    fn lift(&mut self, expr: usize) -> usize {
        let (outer_call, inner_call, lambda) = match self.graph[expr] {
            Node::Call {
                function: inner_call,
                ..
            } => match self.graph[inner_call] {
                Node::Call {
                    function: lambda, ..
                } => match self.graph[lambda] {
                    Node::Lambda { .. } => (expr, inner_call, lambda),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            _ => unreachable!(),
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
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }

        mem::swap(&mut buffer, &mut self.graph[inner_call]);
        mem::swap(&mut buffer, &mut self.graph[outer_call]);

        inner_call
    }

    /// Perform *assoc* inference rule and returns ID of new closure
    fn assoc(&mut self, expr: usize) -> usize {
        let (outer_call, inner_call, lambda) = match self.graph[expr] {
            Node::Call {
                parameter: inner_call,
                ..
            } => match self.graph[inner_call] {
                Node::Call {
                    function: lambda, ..
                } => match self.graph[lambda] {
                    Node::Lambda { .. } => (expr, inner_call, lambda),

                    _ => unreachable!(),
                },

                _ => unreachable!(),
            },
            _ => unreachable!(),
        };

        let mut buffer = mem::replace(
            &mut self.graph[outer_call],
            Node::Consumed("Assoc".to_string()),
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

        inner_call
    }

    fn lift_mfe(&mut self, expr: usize, mfe: usize, expr_depth: usize) -> usize {
        // let name = format!("mfe_extracted_{}", self.fmt_expr(mfe));
        let name = "mfe".to_string();
        let var = self.add_node(Node::Var {
            name: name.clone(),
            kind: VariableKind::Bound {
                depth: expr_depth + 1,
            },
        });
        let lambda = self.add_node(Node::Lambda {
            argument: name,
            body: self.graph.len() + 1,
        });
        self.graph.swap(mfe, var);

        let call = self.add_node(Node::Call {
            function: lambda,
            parameter: var,
        });
        self.graph.swap(call, expr);

        call
    }

    fn is_value(&self, expr: usize) -> bool {
        match self.graph[expr] {
            Node::Lambda { .. } => true,
            Node::Data { tag, .. } => tag.is_value(),
            _ => self.is_structure(expr),
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
    pub fn adjust_depth(&mut self, expr: usize, by: isize) {
        if by == 0 {
            return;
        }
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
                } => {
                    if by > 0 {
                        *depth += by as usize;
                    } else {
                        *depth -= -by as usize;
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    /// Evaluates given expression to an ANSWER
    pub fn evaluate(&mut self, expr: usize, closure_path: &mut ClosurePath) {
        if self.is_debug_enabled() {
            self.add_debug_frame(vec![(expr, "eval")]);
            self.integrity_check();
        }

        match self.graph[expr] {
            Node::Call { function, .. } => {
                self.evaluate(function, closure_path);

                let mut lift_closures_created = 0;
                let mut last_expr = expr;
                loop {
                    let (function, parameter) = match self.graph[last_expr] {
                        Node::Call {
                            parameter,
                            function,
                        } => (function, parameter),
                        _ => unreachable!(),
                    };

                    if self.is_value(function) {
                        if lift_closures_created > 0 {
                            self.adjust_depth(parameter, lift_closures_created);
                        }

                        if let Node::Lambda { body, .. } = self.graph[function] {
                            closure_path.register(last_expr);
                            self.evaluate(body, closure_path);
                            closure_path.backtrack_before_closure(expr);
                        }
                        break;
                    } else {
                        // Closure on function position: LIFT
                        if self.is_debug_enabled() {
                            self.add_debug_frame(vec![(last_expr, "lift")]);
                        }
                        self.lift(last_expr);
                        closure_path.register(last_expr);
                        last_expr = function;
                        lift_closures_created += 1;
                    }
                }
            }
            Node::Var {
                kind: VariableKind::Bound { depth },
                ..
            } => {
                // Compute an answer on parameter position of a binding closure
                {
                    let binding_closure_id = closure_path.get_at_depth(depth);
                    let parameter = match self.graph[binding_closure_id] {
                        Node::Call { parameter, .. } => parameter,
                        _ => unreachable!(),
                    };

                    let rest = closure_path.backtrack_before_closure(binding_closure_id);
                    self.evaluate(parameter, closure_path);
                    closure_path.restore_backtrack(rest);
                }

                let mut assoc_closures_created = 0;
                loop {
                    let binding_closure_id = closure_path.get_at_depth(depth);
                    let (function, parameter) = match self.graph[binding_closure_id] {
                        Node::Call {
                            parameter,
                            function,
                        } => (function, parameter),
                        _ => unreachable!(),
                    };

                    if self.is_value(parameter) {
                        if assoc_closures_created > 0 {
                            self.adjust_depth(function, assoc_closures_created);
                        }

                        // PERF: Lift any MFE found in parameter
                        if let Node::Lambda { body, .. } = &self.graph[parameter] {
                            let mut mfes = vec![];
                            self.find_mfe(parameter, 0, &mut mfes);
                            mfes.retain(|&(id, _)| id != parameter && id != *body);

                            let mfes_count = mfes.len();

                            let mut parameter = parameter;
                            for (mfe, mfe_depth) in mfes {
                                if self.is_debug_enabled() {
                                    self.add_debug_frame(vec![
                                        (mfe, format!("MFE at depth {}", mfe_depth).as_str()),
                                        (parameter, "in expr"),
                                    ]);
                                }
                                // Parameter will gain 1 binder
                                self.adjust_depth(parameter, 1);
                                // But MFE itself will lose some binders (minus the one we just added)
                                self.adjust_depth(mfe, -((mfe_depth + 1) as isize));
                                parameter = self.lift_mfe(parameter, mfe, mfe_depth);
                            }
                            if mfes_count > 0 {
                                return self.evaluate(expr, closure_path);
                            }
                        }

                        // Parameter is a value now, time to deref!
                        if self.is_debug_enabled() {
                            self.add_debug_frame(vec![(expr, "deref"), (parameter, "with this")]);
                        }

                        let cloned_id = self.clone_subtree(parameter);
                        self.adjust_depth(cloned_id, depth as isize);
                        self.graph.swap(expr, cloned_id);
                        break;
                    } else {
                        // Apply assoc if we have closure on parameter position
                        if self.is_debug_enabled() {
                            self.add_debug_frame(vec![
                                (expr, "hole in context"),
                                (parameter, "parameter"),
                                (binding_closure_id, "assoc"),
                            ]);
                        }
                        let new_closure = self.assoc(binding_closure_id);
                        closure_path.register_after_depth(new_closure, depth);
                        assoc_closures_created += 1;
                    }
                }
            }
            Node::Data { tag, .. } if !tag.is_value() => {
                let params = match &self.graph[expr] {
                    Node::Data {
                        constructor_params, ..
                    } => constructor_params.clone(),
                    _ => unreachable!(),
                };
                let result = tag.evaluate(self, closure_path, params);
                self.graph.swap(expr, result);
            }
            _ => {} // Everything else is already a value
        }
    }

    pub fn evaluate_root(&mut self) {
        self.evaluate(self.root, &mut ClosurePath::new());
    }

    pub fn unwrap_closure_chain(&self, expr: usize) -> usize {
        if let Node::Call { function, .. } = self.graph[expr] {
            if let Node::Lambda { body, .. } = &self.graph[function] {
                return self.unwrap_closure_chain(*body);
            }
        };

        expr
    }

    fn find_mfe(
        &self,
        expr: usize,
        root_depth: usize,
        results: &mut Vec<(usize, usize)>,
    ) -> Option<usize> {
        match &self.graph[expr] {
            // Locally bound variables can't be included in MFE
            Node::Var {
                kind: VariableKind::Bound { depth },
                ..
            } if *depth <= root_depth => None,
            Node::Call {
                function,
                parameter,
            } => {
                let function_mfe = self.find_mfe(*function, root_depth, results);
                let param_mfe = self.find_mfe(*parameter, root_depth, results);

                Option::zip(function_mfe, param_mfe).map(|(f, p)| {
                    // If both parameter and function are MFEs,
                    // remove them from results and add current expr instead
                    for value_to_remove in [f, p] {
                        if let Some(pos) = results.iter().position(|&(r, _)| r == value_to_remove) {
                            results.remove(pos);
                        };
                    }

                    results.push((expr, root_depth));
                    expr
                })
            }
            Node::Lambda { body, .. } => self.find_mfe(*body, root_depth + 1, results),
            // Everything else can be a part of MFE
            _ => Some(expr),
        }
    }
}
