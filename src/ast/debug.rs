use crate::ast::{DebugNode, Edge, Node, Primitive, VariableKind, AST};

impl AST {
    fn dot_node_with_attributes(
        id: usize,
        label: &str,
        color: &'static str,
        font_color: &'static str,
    ) -> String {
        format!("{id} [label=\"{label}\" style=filled fillcolor={color} fontcolor={font_color}]")
    }
    /// Convert current Graph state into String in DOT format.
    /// This can be then rendered using graphviz into PNG/SVG for analysis and debugging.
    /// You can pass a list of additional labels for nodes.
    /// Also see debug.html for interactive DOT viewer
    pub fn to_dot(&self) -> String {
        use std::fmt::Write;

        let mut result = String::from("digraph EXPR {\n");

        for node_id in self.graph.node_indices() {
            let id = node_id.index();
            match self.graph.node_weight(node_id).unwrap() {
                Node::Lambda { argument_name } => writeln!(
                    result,
                    "{}",
                    Self::dot_node_with_attributes(
                        id,
                        &format!("λ{argument_name}"),
                        "green",
                        "white"
                    )
                )
                .unwrap(),
                Node::Closure { argument_name } => {
                    writeln!(
                        result,
                        "{}",
                        Self::dot_node_with_attributes(
                            id,
                            &format!("let {argument_name} in"),
                            "red",
                            "white"
                        )
                    )
                    .unwrap();
                    let parameter = self.follow_edge(node_id, Edge::Parameter).unwrap().index();
                    let body = self.follow_edge(node_id, Edge::Body).unwrap().index();
                    // Group function and parameter on same rank
                    writeln!(result, "{{ rank = same; {body}; {parameter}; }}").unwrap();
                    // Force horizontal order: function on the left, parameter on the right
                    writeln!(result, "{body} -> {parameter} [style=invis]").unwrap();
                }
                Node::Application => {
                    writeln!(
                        result,
                        "{}",
                        Self::dot_node_with_attributes(id, &"call".to_string(), "blue", "white")
                    )
                    .unwrap();
                    let parameter = self.follow_edge(node_id, Edge::Parameter).unwrap().index();
                    let function = self.follow_edge(node_id, Edge::Function).unwrap().index();
                    // Group function and parameter on same rank
                    writeln!(result, "{{ rank = same; {function}; {parameter}; }}").unwrap();
                    // Force horizontal order: function on the left, parameter on the right
                    writeln!(result, "{function} -> {parameter} [style=invis]").unwrap();
                }
                Node::Variable(kind) => writeln!(
                    result,
                    "{}",
                    Self::dot_node_with_attributes(
                        id,
                        self.get_variable_name(node_id).unwrap(),
                        match kind {
                            VariableKind::Bound => "gray",
                            VariableKind::Free(_) => "orange",
                        },
                        "white"
                    )
                )
                .unwrap(),
                Node::Data { tag } => {
                    writeln!(
                        result,
                        "{id} [label=\"{id}: Data {}\"]",
                        String::try_from(*tag).unwrap()
                    )
                    .unwrap();
                }
                Node::Primitive(Primitive::Bytes(bytes)) => writeln!(
                    result,
                    "{id} [label=\"Bytes: {}\"]",
                    str::from_utf8(bytes).unwrap()
                )
                .unwrap(),
                Node::Primitive(value) => {
                    writeln!(result, "{id} [label=\"{id}: {:?}\"]", value).unwrap()
                }
                Node::Debug(DebugNode::Annotation { text }) => {
                    writeln!(result, "{id} [label=\"{id}: {text}\", color=\"red\"]",).unwrap()
                }
            }
        }

        for edge_id in self.graph.edge_indices() {
            let edge = self.graph.edge_weight(edge_id).unwrap();
            if let Node::Variable(_) | Node::Data { .. } = self
                .graph
                .node_weight(self.graph.edge_endpoints(edge_id).unwrap().0)
                .unwrap()
            {
            } else {
                let (from, to) = self.graph.edge_endpoints(edge_id).unwrap();
                let from = from.index();
                let to = to.index();
                writeln!(result, "{from} -> {to} [label=\"{:?}\"]", edge).unwrap();
            }
        }

        writeln!(result, "}}").unwrap();
        result
    }
}
