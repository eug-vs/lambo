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
    fn if_then_else(condition: Expr, then: Expr, r#else: Expr) -> Expr {
        Expr::from_str("(( condition then ) else )")
            .provide_variable("condition", condition)
            .provide_variable("then", then)
            .provide_variable("else", r#else)
    }
}

impl PartialEq for Expr {
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

    fn evaluate(&self) -> Self {
        let previous_str = format!("{}", self);
        let result = match self {
            Expr::Call(function, argument) => {
                let evaluated_argument = argument.evaluate();
                match function.evaluate() {
                    Expr::Lambda(_arg, body) => {
                        body.beta_reduce(&evaluated_argument, 1).evaluate() // We start from 1 (see above)
                    }
                    // Special case for OnE AnD oNlY built-in beta-equivalence operator
                    Expr::Call(operator, right) => {
                        if !match operator.evaluate() {
                            Expr::Var(var) => var.name == String::from("#eq"),
                            _ => false,
                        } {
                            return self.clone();
                        }

                        // Evaluate both and check alpha-equivalence
                        if right.evaluate() == evaluated_argument {
                            Self::TRUE()
                        } else {
                            Self::FALSE()
                        }
                    }
                    _ => self.clone(), // Maybe beta reduce here as well?
                }
            }
            expr => expr.clone(),
        };
        let new_str = format!("{}", result);
        if previous_str != new_str {
            // println!("Evaluated: {} => {}", self, result);
        }
        result
    }
    fn beta_reduce(&self, replace_with: &Expr, depth: usize) -> Expr {
        match self {
            Expr::Var(var) => match var.kind {
                VariableKind::Bound(d) => {
                    if d == depth {
                        replace_with.clone()
                    } else {
                        self.clone()
                    }
                }
                _ => self.clone(),
            },
            Expr::Lambda(arg_name, body) => {
                let new_body = body.beta_reduce(replace_with, depth + 1);
                Self::Lambda(arg_name.clone(), Box::new(new_body))
            }
            Expr::Call(func, arg) => {
                let new_func = func.beta_reduce(replace_with, depth);
                let new_arg = arg.beta_reduce(replace_with, depth);
                Self::Call(Box::new(new_func), Box::new(new_arg))
            }
        }
    }
    fn evaluate_scoped(&self, context: &HashMap<String, Expr>) -> Expr {
        // TODO: only include functions from context that are actually used
        let expr = context.iter().fold(self.clone(), |acc, (name, value)| {
            acc.provide_variable(name, value.clone())
        });
        expr.evaluate().replace_from_context(&context)
    }
    fn replace_from_context(&self, context: &HashMap<String, Expr>) -> Expr {
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
    let mut context = HashMap::<String, Expr>::new();

    for line in extract_from_markdown()
        .iter()
        .filter(|line| !line.starts_with("//") && line.len() > 0)
    {
        let mut words = line.split(&[' ', '\t']);
        match words.next().unwrap() {
            "eval" => {
                let expr = Expr::from_str(&words.collect::<Vec<_>>().join(" "));
                println!("{}\n => {}", expr, expr.evaluate_scoped(&context))
            }
            "let" => {
                let variable_name = words.next().unwrap();
                let expr = Expr::from_str(&words.collect::<Vec<_>>().join(" "));
                context.insert(variable_name.to_string(), expr);
            }
            "assert" => {
                let remaining = words.collect::<Vec<_>>().join(" ");
                let mut chars = remaining.chars().peekable();
                let left = Expr::from_chars(&mut chars).evaluate_scoped(&context);
                let right = Expr::from_chars(&mut chars).evaluate_scoped(&context);
                assert_eq!(left, right);
            }
            _ => panic!("Invalid syntax"),
        }
    }
}
