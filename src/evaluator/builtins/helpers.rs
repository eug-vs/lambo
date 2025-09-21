use crate::evaluator::{builtins::ConstructorTag, reduction::ClosurePath, Graph, Node};

#[derive(Debug, Clone, Copy)]
pub enum HelperFunctionTag {
    /// Meta-constructor to create constructors at runtime
    CreateConstructor,
    Match,
}

impl HelperFunctionTag {
    pub fn argument_names(&self) -> Vec<&'static str> {
        match self {
            Self::CreateConstructor => vec!["arity"],
            Self::Match => vec!["constructor", "transform", "fallback", "value"],
        }
    }

    pub fn evaluate(
        &self,
        graph: &mut Graph,
        closure_path: &mut ClosurePath,
        arguments: Vec<usize>,
    ) -> usize {
        match self {
            Self::CreateConstructor => {
                let [arity_id] = *arguments.as_slice() else {
                    graph.panic("Incorrect argument count for CreateConstructor")
                };
                graph.evaluate(arity_id, closure_path);

                let arity = graph.graph[arity_id]
                    .extract_primitive_number()
                    .expect("Expected number for arity");

                let tag = ConstructorTag::CustomTag {
                    uid: graph.generate_uid(),
                    arity,
                };

                graph.add_constructor(tag)
            }
            Self::Match => {
                let [constructor, transform, fallback, value] = *arguments.as_slice() else {
                    graph.panic("Incorrect argument count for Match")
                };
                graph.evaluate(constructor, closure_path);
                graph.evaluate(value, closure_path);

                let (value_tag_uid, value_contents) = match &graph.graph[value] {
                    Node::Data {
                        tag: ConstructorTag::CustomTag { uid, .. },
                        constructor_params,
                    } => (*uid, constructor_params.clone()),
                    _ => graph.panic("Can not use match on built-in tags!"),
                };

                let constructor_tag_uid = {
                    let mut node = constructor;
                    while let Node::Call { function: next, .. } | Node::Lambda { body: next, .. } =
                        graph.graph[node]
                    {
                        node = next
                    }
                    match graph.graph[node] {
                        Node::Data {
                            tag: ConstructorTag::CustomTag { uid, .. },
                            ..
                        } => uid,
                        _ => graph.panic("Incorrect constructor passed to Match"),
                    }
                };

                let result = if constructor_tag_uid == value_tag_uid {
                    // Call transform function with extracted contents
                    value_contents.iter().fold(transform, |acc, &parameter| {
                        graph.add_node(Node::Call {
                            function: acc,
                            parameter,
                        })
                    })
                } else {
                    // Call fallback function with value again
                    // Such API allows easier chaining of #match expressions
                    graph.add_node(Node::Call {
                        function: fallback,
                        parameter: value,
                    })
                };

                graph.evaluate(result, closure_path);
                result
            }
        }
    }
}
