use std::{
    fmt::Display,
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    mem,
};

use crate::{
    evaluator::{EvaluationOrder, Graph},
    runtime::IOMonad,
};
mod evaluator;
mod parser;
mod runtime;

#[derive(Debug, Clone)]
enum VariableKind {
    /// Represents a De Brujin index
    Bound {
        depth: usize,
    },
    Free,
}

#[derive(Debug, Clone)]
enum Expr {
    Var {
        name: String,
        kind: VariableKind,
    },
    Lambda {
        argument: String,
        body: Box<Expr>,
    },
    Call {
        function: Box<Expr>,
        parameter: Box<Expr>,
    },
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Var { name, .. } => write!(f, "{}", name),
            Expr::Lambda { argument, body, .. } => {
                write!(f, "λ{}", argument)?;
                write!(f, ".")?;
                body.fmt(f)
            }
            Expr::Call {
                function,
                parameter,
                ..
            } => {
                write!(f, "(")?;
                match &**function {
                    Expr::Lambda { .. } => {
                        write!(f, "(")?;
                        function.fmt(f)?;
                        write!(f, ")")?;
                    }
                    _ => function.fmt(f)?,
                }
                write!(f, " ")?;

                // We can avoid wrapping argument in (),
                // even if it's a Lambda, because closing paren follows anyway
                parameter.fmt(f)?;
                write!(f, ")")
            }
        }
    }
}
impl Expr {
    fn fmt_de_brujin(&self) -> String {
        match self {
            Expr::Var {
                name,
                kind: VariableKind::Free,
                ..
            } => format!("{}", name),
            Expr::Var {
                kind: VariableKind::Bound { depth, .. },
                ..
            } => format!("{}", depth),
            Expr::Lambda { body, .. } => format!("λ {}", body.fmt_de_brujin()),
            Expr::Call {
                function,
                parameter,
                ..
            } => format!(
                "({} {})",
                function.fmt_de_brujin(),
                parameter.fmt_de_brujin()
            ),
        }
    }
}

impl Expr {
    #[allow(non_snake_case)]
    fn TRUE() -> Expr {
        Expr::from_str("λx.λy.x")
    }
    #[allow(non_snake_case)]
    fn FALSE() -> Expr {
        Expr::from_str("λx.λy.y")
    }
}

impl PartialEq for Expr {
    /// Alpha equivalence
    fn eq(&self, other: &Self) -> bool {
        self.fmt_de_brujin() == other.fmt_de_brujin()
    }
}

impl Expr {
    /// Just a semantic sugar on top of existing lambda syntax
    fn provide_variable(&self, variable_name: &str, value: Expr) -> Self {
        let formatted = format!("(@{variable_name}.{self}) {value}");
        Self::from_str(&formatted)
    }
    // AKA apply
    fn evaluate(&mut self) {
        let debug_path = {
            let mut hasher = DefaultHasher::new();
            self.fmt_de_brujin().hash(&mut hasher);
            let hash = hasher.finish();
            format!("./debug/{}", hash)
        };

        // Get ownership of Expr, temporarily replacing &self with dummy value
        let expr = mem::replace(
            self,
            Expr::Var {
                name: "#evaluate_in_progress".to_string(),
                kind: VariableKind::Free,
            },
        );
        let mut graph = Graph::from_expr(expr, false);
        graph.evaluate(graph.root, EvaluationOrder::Lazy);
        graph.dump_debug_frames(&debug_path);

        *self = graph.to_expr(graph.root);
    }

    fn scoped(&self, context: &Vec<(String, Expr)>) -> Expr {
        // TODO: only include functions from context that are actually used
        context
            .iter()
            .rev() // WARN: reverse iterator to apply in the right order
            .fold(self.clone(), |acc, (name, value)| {
                acc.provide_variable(name, (*value).clone())
            })
    }
    fn replace_from_context(&self, context: &Vec<(String, Expr)>) -> Expr {
        for (name, value) in context {
            if value == self {
                return Expr::Var {
                    name: name.clone(),
                    kind: VariableKind::Free,
                };
            }
        }
        match self {
            Expr::Lambda { body, argument } => Expr::Lambda {
                argument: argument.to_string(),
                body: Box::new(body.replace_from_context(context)),
            },
            Expr::Call {
                function,
                parameter,
            } => Expr::Call {
                function: Box::new(function.replace_from_context(context)),
                parameter: Box::new(parameter.replace_from_context(context)),
            },
            Expr::Var { .. } => self.clone(),
        }
    }
}

fn extract_from_markdown() -> Vec<String> {
    let input = fs::read_to_string("./README.md").unwrap();
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for line in input.lines() {
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            lines.push(line.to_string());
        }
    }

    lines
}

fn main() {
    let mut context = vec![];

    let extracted = extract_from_markdown();
    let mut lines = extracted
        .iter()
        .map(|line| line.split("//").next().unwrap_or(""))
        .filter(|line| line.trim().len() > 0);

    loop {
        let input = match lines.next() {
            Some(line) => {
                // Handle multiline statements with [ ]
                if line.contains("[") {
                    let mut joined = line.to_string().replace("[", "");
                    while let Some(l) = lines.next() {
                        joined = joined + "\n" + &l.replace("]", "");
                        if l.contains("]") {
                            break;
                        }
                    }
                    joined
                } else {
                    line.to_string()
                }
            }
            _ => break,
        };
        let mut words = input.split(&[' ', '\t']).peekable();
        match words.peek().unwrap() {
            &"let" => {
                words.next();
                let variable_name = words.next().unwrap();
                let expr = Expr::from_str(&words.collect::<Vec<_>>().join(" "));
                context.push((variable_name.to_string(), expr));
            }
            _ => {
                let input = &words.collect::<Vec<_>>().join(" ");
                println!();
                println!("$   {}", input);
                let mut expr = Expr::from_str(input).scoped(&context);
                // println!("~   {:?}", expr);
                expr.evaluate();
                println!("=>  {}", expr.replace_from_context(&context));

                match IOMonad::from_expr(&expr) {
                    Some(Ok(mut monad)) => match monad.unwrap() {
                        Ok(runtime_result) => println!("==> {}", runtime_result),
                        Err(msg) => println!("Runtime error: {}", msg),
                    },
                    Some(Err(msg)) => println!("Could not parse IO: {}", msg),
                    None => {} // Not an IO, skipping runtime
                }
            }
        }
    }
}
