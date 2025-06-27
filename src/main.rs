use std::{
    fmt::Display,
    fs,
    io::{Write, stdout},
};
mod parser;

#[derive(Debug, Clone)]
enum VariableKind {
    /// Represents a De Brujin index
    Bound(usize),
    Free,
}

#[derive(Debug, Clone)]
struct Variable {
    name: String,
    kind: VariableKind,
}

impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone)]
enum Expr {
    Var(Variable),
    Lambda(String, Box<Expr>),
    Call(Box<Expr>, Box<Expr>),
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Var(name) => write!(f, "{}", name),
            Expr::Lambda(argument, body) => {
                write!(f, "λ{}.", argument)?;
                body.fmt(f)
            }
            Expr::Call(function, argument) => {
                write!(f, "(")?;
                match **function {
                    Expr::Lambda(_, _) => {
                        write!(f, "(")?;
                        function.fmt(f)?;
                        write!(f, ")")?;
                    }
                    _ => function.fmt(f)?,
                }
                write!(f, " ")?;

                // We can avoid wrapping argument in (),
                // even if it's a Lambda, because closing paren follows anyway
                argument.fmt(f)?;
                write!(f, ")")
            }
        }
    }
}
impl Expr {
    fn fmt_de_brujin(&self) -> String {
        match self {
            Expr::Var(variable) => match variable.kind {
                VariableKind::Free => format!("{}", variable.name),
                VariableKind::Bound(depth) => format!("{}", depth),
            },
            Expr::Lambda(_argument, body) => format!("λ {}", body.fmt_de_brujin()),
            Expr::Call(function, argument) => format!(
                "({} {})",
                function.fmt_de_brujin(),
                argument.fmt_de_brujin()
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
    /// Performs an adjustment to variables' depths.
    /// Always call with `cutoff=1` initially
    fn adjust_depth(&self, cutoff: usize, by: isize) -> Expr {
        match self {
            Expr::Var(var) => match var.kind {
                VariableKind::Bound(d) => {
                    if d >= cutoff {
                        Expr::Var(Variable {
                            name: var.name.clone(),
                            kind: VariableKind::Bound(((d as isize) + by) as usize),
                        })
                    } else {
                        self.clone()
                    }
                }
                _ => self.clone(),
            },
            Expr::Lambda(arg_name, body) => {
                let adjusted_body = body.adjust_depth(cutoff + 1, by);
                Expr::Lambda(arg_name.clone(), Box::new(adjusted_body))
            }
            Expr::Call(func, arg) => {
                let adjusted_func = func.adjust_depth(cutoff, by);
                let adjusted_arg = arg.adjust_depth(cutoff, by);
                Expr::Call(Box::new(adjusted_func), Box::new(adjusted_arg))
            }
        }
    }

    fn handle_builtin_functions(function: &Expr, argument: &Expr) -> Option<Expr> {
        match function {
            // Beta-equivalence operator: #eq
            Expr::Call(operator, right) => {
                match operator.evaluate() {
                    Expr::Var(var) => {
                        if var.name == String::from("#eq") {
                            // Evaluate both and check alpha-equivalence
                            if right.evaluate() == *argument {
                                return Some(Self::TRUE());
                            } else {
                                return Some(Self::FALSE());
                            }
                        }
                    }
                    _ => {}
                };
            }
            _ => {}
        }
        None
    }

    fn evaluate(&self) -> Self {
        match self {
            Expr::Call(function, argument) => {
                let evaluated_argument = argument.evaluate();
                let evaluated_function = function.evaluate();

                match Self::handle_builtin_functions(&function, &argument) {
                    Some(result) => return result,
                    None => {}
                }

                match evaluated_function.clone() {
                    Expr::Lambda(_arg, body) => {
                        // We start from 1 (see above)
                        return body
                            .substitute(&evaluated_argument.adjust_depth(1, 1), 1)
                            .adjust_depth(1, -1)
                            .evaluate();
                    }
                    _ => {}
                };
                Expr::Call(Box::new(evaluated_function), Box::new(evaluated_argument))
            }
            Expr::Lambda(name, body) => {
                let evaluated_body = body.evaluate();
                Self::Lambda(name.clone(), Box::new(evaluated_body))
            }
            expr => expr.clone(),
        }
    }
    fn substitute(&self, with: &Expr, at_depth: usize) -> Expr {
        match self {
            Expr::Var(var) => match var.kind {
                VariableKind::Bound(d) => {
                    if d == at_depth {
                        with.clone()
                    } else {
                        self.clone()
                    }
                }
                _ => self.clone(),
            },
            Expr::Lambda(arg_name, body) => {
                let new_body = body.substitute(&with.adjust_depth(1, 1), at_depth + 1);
                Self::Lambda(arg_name.clone(), Box::new(new_body))
            }
            Expr::Call(func, arg) => {
                let new_func = func.substitute(with, at_depth);
                let new_arg = arg.substitute(with, at_depth);
                Self::Call(Box::new(new_func), Box::new(new_arg))
            }
        }
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
                return Expr::Var(Variable {
                    name: name.clone(),
                    kind: VariableKind::Free,
                });
            }
        }
        match self {
            Expr::Lambda(arg, body) => {
                let new_body = body.replace_from_context(context);
                Expr::Lambda(arg.clone(), Box::new(new_body))
            }
            Expr::Call(func, arg) => {
                let func = func.replace_from_context(context);
                let arg = arg.replace_from_context(context);
                Expr::Call(Box::new(func), Box::new(arg))
            }
            Expr::Var(_) => self.clone(),
        }
    }
    fn check_assertions(&self) {
        match self {
            Expr::Call(func, arg) => {
                func.check_assertions();
                arg.check_assertions();

                match *(*func).clone() {
                    Expr::Var(var) => {
                        if var.name == "assert" {
                            return assert!(
                                **arg == Self::TRUE(),
                                "Assertion failed: {} expected to be {}",
                                arg,
                                Self::TRUE()
                            );
                        }
                    }
                    _ => {}
                }
            }
            Expr::Lambda(_, body) => {
                body.check_assertions();
            }
            _ => {}
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

    for line in extract_from_markdown()
        .iter()
        .filter(|line| !line.starts_with("//") && line.len() > 0)
    {
        let mut words = line.split(&[' ', '\t']).peekable();
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
                let expr = Expr::from_str(input).scoped(&context);
                // println!("~   {}", expr);
                let result = expr.evaluate();
                println!("=>  {}", result.replace_from_context(&context));

                result.check_assertions();
            }
        }
    }
}
