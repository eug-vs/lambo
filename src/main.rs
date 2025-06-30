use std::{fmt::Display, fs, io::stdin};

use crate::runtime::IOMonad;
mod parser;
mod runtime;

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
                match &**operator {
                    Expr::Var(var) => {
                        match var.name.as_str() {
                            "#eq" => {
                                // Compare beta-equivalence
                                if right.evaluate_normal() == argument.evaluate_normal() {
                                    return Some(Self::TRUE());
                                } else {
                                    return Some(Self::FALSE());
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                };
            }
            Expr::Var(var) => match var.name.as_str() {
                "#compile" => return Some(argument.evaluate_normal()),
                _ => {}
            },
            _ => {}
        }
        None
    }
    /// Normal Order (as per definition, with lambda abstraction folding)
    /// This evaluation order guarantees to converge to a normal form if it exists.
    fn evaluate_normal(&self) -> Expr {
        match self {
            Expr::Call(func, argument) => {
                let evaluated_func = func.evaluate_normal();
                // disable side effects
                match Self::handle_builtin_functions(&evaluated_func, &argument) {
                    Some(result) => return result,
                    None => {}
                }

                match evaluated_func {
                    Expr::Lambda(_arg, body) => {
                        // We start from 1 (see above)
                        return body
                            .substitute(&argument.adjust_depth(1, 1), 1)
                            .adjust_depth(1, -1)
                            .evaluate_normal();
                    }
                    Expr::Var(_) => {
                        // Evaluated function is just a variable - can not substitute.
                        // But we still have to reduce down to normal form, so evaluate argument
                        Expr::Call(
                            Box::new(evaluated_func),
                            Box::new(argument.evaluate_normal()),
                        )
                    }
                    // Call is no longer reducible, already in normal form
                    _ => self.clone(),
                }
            }
            Expr::Lambda(name, body) => {
                Expr::Lambda(name.clone(), Box::new(body.evaluate_normal()))
            }
            _ => self.clone(),
        }
    }

    /// Evaluate expression using Non-Strict Order strategy (Call by Name, aka Lazy)
    fn evaluate(&self) -> Self {
        match self {
            Expr::Call(function, argument) => {
                let evaluated_function = function.evaluate();
                match Self::handle_builtin_functions(&evaluated_function, &argument) {
                    Some(result) => return result,
                    None => {}
                }
                match evaluated_function {
                    Expr::Lambda(_arg, body) => {
                        // We start from 1 (see above)
                        body.substitute(&argument.adjust_depth(1, 1), 1)
                            .adjust_depth(1, -1)
                            .evaluate()
                    }
                    Expr::Var(_) => {
                        // Evaluated function is just a variable - can not substitute.
                        // But we still have to reduce down to normal form, so evaluate argument
                        Expr::Call(Box::new(evaluated_function), Box::new(argument.evaluate()))
                    }
                    // Call is no longer reducible, already in normal form
                    _ => self.clone(),
                }
            }
            // Since we are doing "Call by Name" evaluation, we do not collapse Lambda body, i.e
            // λx.(SOME_HARD_TO_COMPUTE_FUNCTION)
            // will not get evaluated until it's actually called
            _ => self.clone(),
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
        .map(|line| line.split("//").next().unwrap())
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
                let runtime_result = IOMonad::from_expr(&result).unwrap();
                println!("==> {}", runtime_result);
            }
        }
    }
}
