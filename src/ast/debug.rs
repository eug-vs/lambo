use crate::ast::{AST, DebugNode, Edge, Node, Primitive, VariableKind, builtins::ConstructorTag};

impl AST {
    fn dot_node_with_attributes(
        id: usize,
        label: &str,
        color: &'static str,
        font_color: &'static str,
        tooltip: Option<&str>,
    ) -> String {
        let id = id.to_string();
        let tooltip = tooltip.unwrap_or(&id);
        format!(
            "{id} [label=\"{label}\" style=filled fillcolor={color} fontcolor={font_color} tooltip=\"{tooltip}\"]"
        )
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
                        "white",
                        None
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
                            "white",
                            None
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
                        Self::dot_node_with_attributes(
                            id,
                            &"call".to_string(),
                            "blue",
                            "white",
                            None
                        )
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
                        self.get_variable_name(node_id)
                            .unwrap_or(&String::from("PENDING_REMOVAL")),
                        match kind {
                            VariableKind::Bound => "gray",
                            VariableKind::Free(_) => "orange",
                        },
                        "white",
                        self.follow_edge(node_id, Edge::Binder(0))
                            .map(|binder| format!("Bound to {}", binder.index()))
                            .ok()
                            .as_deref()
                    )
                )
                .unwrap(),
                Node::Data { tag } => {
                    let binders = ConstructorTag::get_binders(self, node_id);
                    let argument_names = tag.argument_names();

                    writeln!(
                        result,
                        "{}",
                        Self::dot_node_with_attributes(
                            id,
                            &format!(
                                "Data {} ({}/{})",
                                String::from(*tag),
                                binders.len(),
                                argument_names.len(),
                            ),
                            "yellow",
                            "black",
                            Some(
                                &binders
                                    .into_iter()
                                    .zip(argument_names)
                                    .map(|(binder, name)| format!("{}: {}", name, binder.index()))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            )
                        )
                    )
                    .unwrap();
                }
                Node::Primitive(Primitive::Bytes(bytes)) => writeln!(
                    result,
                    "{id} [label=\"Bytes: {}\"]",
                    str::from_utf8(bytes).unwrap()
                )
                .unwrap(),
                Node::Primitive(value) => writeln!(result, "{id} [label=\"{:?}\"]", value).unwrap(),
                Node::Debug(DebugNode::Annotation { text }) => {
                    writeln!(result, "{id} [label=\"{text}\", color=\"red\"]",).unwrap()
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
