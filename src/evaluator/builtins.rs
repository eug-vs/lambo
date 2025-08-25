use crate::{
    evaluator::{Graph, Node},
    Expr, VariableKind,
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
}
