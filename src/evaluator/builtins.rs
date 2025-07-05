use std::mem;

use crate::{
    Expr, VariableKind,
    evaluator::{EvaluationOrder, Graph, Node},
};

impl Graph {
    fn insert_boolean(&mut self, value: bool) -> usize {
        self.add_expr_to_graph(if value { Expr::TRUE() } else { Expr::FALSE() })
    }

    fn is_alpha_equivalent(&self, left: usize, right: usize) -> bool {
        match &self.graph[left] {
            Node::Var {
                name: left_name,
                kind: left_kind,
            } => match &self.graph[right] {
                Node::Var {
                    name: right_name,
                    kind: right_kind,
                } => match left_kind {
                    VariableKind::Free => match right_kind {
                        VariableKind::Free => left_name == right_name,
                        _ => false,
                    },
                    VariableKind::Bound {
                        depth: left_depth, ..
                    } => match right_kind {
                        VariableKind::Bound {
                            depth: right_depth, ..
                        } => left_depth == right_depth,
                        _ => false,
                    },
                },
                _ => false,
            },
            Node::Call {
                function: left_fn,
                parameter: left_param,
            } => match self.graph[right] {
                Node::Call {
                    function: right_fn,
                    parameter: right_param,
                } => {
                    self.is_alpha_equivalent(*left_fn, right_fn)
                        && self.is_alpha_equivalent(*left_param, right_param)
                }
                _ => false,
            },
            Node::Lambda {
                body: left_body, ..
            } => match self.graph[right] {
                Node::Lambda {
                    body: right_body, ..
                } => self.is_alpha_equivalent(*left_body, right_body),
                _ => false,
            },
            Node::Consumed(_) => self.panic_consumed_node(left),
        }
    }

    pub fn handle_builtins(&mut self, id: usize) -> bool {
        match self.graph[id] {
            Node::Call {
                function,
                parameter: right,
            } => match self.graph[function] {
                Node::Call {
                    function,
                    parameter: left,
                } => match &self.graph[function] {
                    Node::Var {
                        name,
                        kind: VariableKind::Free,
                    } if name == "#eq" => {
                        self.evaluate(left, EvaluationOrder::Normal);
                        self.evaluate(right, EvaluationOrder::Normal);
                        let result = self.is_alpha_equivalent(left, right);
                        let result_id = self.insert_boolean(result);
                        self.graph[id] = mem::replace(
                            &mut self.graph[result_id],
                            Node::Consumed("By #eq".to_string()),
                        );
                        if self.debug {
                            self.debug_frames
                                .push(self.to_dot(vec![(id, "resolved #eq")]));
                        }
                        return true;
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
        false
    }
}
