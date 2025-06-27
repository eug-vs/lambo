use std::{collections::HashMap, fmt::Display, fs, iter::Peekable};

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
                function.fmt(f)?;
                write!(f, " ")?;
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
    fn from_str(s: &str) -> Self {
        Self::from_chars(&mut s.chars().peekable())
    }
    fn variable_name_from_chars<I: Iterator<Item = char>>(iterator: &mut Peekable<I>) -> String {
        let mut chars = vec![];
        loop {
            match iterator.peek() {
                Some(char) => match char {
                    ' ' | '(' | ')' | '.' | '@' | 'λ' => break,
                    c => chars.push(*c),
                },
                None => break,
            }
            iterator.next().unwrap();
        }
        chars.iter().collect::<String>()
    }
    fn consume_whitespace<I: Iterator<Item = char>>(iterator: &mut Peekable<I>) {
        loop {
            match iterator.peek().unwrap() {
                ' ' | '\n' => {}
                _ => break,
            }
            iterator.next();
        }
    }

    /// Just a semantic sugar on top of existing lambda syntax
    fn provide_variable(&self, variable_name: &str, value: Expr) -> Self {
        Self::from_str(format!("( @{variable_name}.{self} {value})").as_str())
    }
    fn from_chars<I: Iterator<Item = char>>(iterator: &mut Peekable<I>) -> Self {
        let ctx = vec![];
        Self::from_chars_inner(iterator, ctx)
    }
    fn from_chars_inner<I: Iterator<Item = char>>(
        iterator: &mut Peekable<I>,
        mut ctx: Vec<String>,
    ) -> Self {
        Self::consume_whitespace(iterator);
        match iterator.peek().unwrap() {
            '(' => {
                iterator.next().unwrap();

                Self::consume_whitespace(iterator);

                let func = Self::from_chars_inner(iterator, ctx.clone());

                Self::consume_whitespace(iterator);

                let arg = Self::from_chars_inner(iterator, ctx);

                Self::consume_whitespace(iterator);

                let paren = iterator.next().unwrap();
                assert_eq!(paren, ')');
                Expr::Call(Box::new(func), Box::new(arg))
            }
            'λ' | '@' => {
                iterator.next().unwrap();
                let var = Self::variable_name_from_chars(iterator);
                ctx.push(var.clone());

                let dot = iterator.next().unwrap();
                assert_eq!(dot, '.');

                let body = Self::from_chars_inner(iterator, ctx);
                Expr::Lambda(var, Box::new(body))
            }
            _ => {
                let name = Self::variable_name_from_chars(iterator);
                let kind = match ctx.iter().rev().position(|n| *n == name) {
                    Some(depth) => VariableKind::Bound(depth + 1), // Just to avoid 0, purely sugar
                    None => VariableKind::Free,
                };
                Expr::Var(Variable { name, kind })
            }
        }
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

    fn evaluate(&self) -> Self {
        match self {
            Expr::Call(function, argument) => {
                let evaluated_argument = argument.evaluate();
                let evaluated_function = function.evaluate();

                match evaluated_function.clone() {
                    Expr::Lambda(_arg, body) => {
                        // We start from 1 (see above)
                        return body
                            .substitute(&evaluated_argument.adjust_depth(1, 1), 1)
                            .adjust_depth(1, -1)
                            .evaluate();
                    }
                    // Special case for OnE AnD oNlY built-in beta-equivalence operator
                    Expr::Call(operator, right) => {
                        match operator.evaluate() {
                            Expr::Var(var) => {
                                if var.name == String::from("#eq") {
                                    // Evaluate both and check alpha-equivalence
                                    if right.evaluate() == evaluated_argument {
                                        return Self::TRUE();
                                    } else {
                                        return Self::FALSE();
                                    }
                                }
                            }
                            _ => {}
                        };
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
        self.clone() // TODO: recursively replace_from_context further down
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
        let mut words = line.split(&[' ', '\t']);
        match words.next().unwrap() {
            "eval" => {
                let expr = Expr::from_str(&words.collect::<Vec<_>>().join(" "));
                println!(
                    "\n{}\n => {}",
                    expr,
                    expr.scoped(&context)
                        .evaluate()
                        .replace_from_context(&context)
                )
            }
            "let" => {
                let variable_name = words.next().unwrap();
                let expr = Expr::from_str(&words.collect::<Vec<_>>().join(" "));
                context.push((variable_name.to_string(), expr));
            }
            "assert" => {
                let remaining = words.collect::<Vec<_>>().join(" ");
                let mut chars = remaining.chars().peekable();
                let left = Expr::from_chars(&mut chars).scoped(&context);
                let right = Expr::from_chars(&mut chars).scoped(&context);

                let left_eval = left.evaluate();
                let right_eval = right.evaluate();
                assert_eq!(
                    left_eval,
                    right_eval,
                    "Assertion failed on line: {}\nLeft: {} => {}\nRight: {} => {}",
                    line,
                    left.fmt_de_brujin(),
                    left_eval.fmt_de_brujin(),
                    right.fmt_de_brujin(),
                    right_eval.fmt_de_brujin()
                );
            }
            _ => panic!("Invalid syntax"),
        }
    }
}
