use std::{fmt::Display, iter::Peekable};

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

// impl Debug for Variable {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self.kind {
//             VariableKind::Free => write!(f, "{}", self.name),
//             VariableKind::Bound(depth) => write!(f, "{}_{}", self.name, depth),
//         }
//     }
// }

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

impl Expr {
    fn from_str(s: &str) -> Self {
        Self::from_chars(&mut s.chars().peekable())
    }
    fn variable_name_from_chars<I: Iterator<Item = char>>(iterator: &mut Peekable<I>) -> String {
        let mut chars = vec![];
        loop {
            match iterator.peek() {
                Some(char) => match char {
                    ' ' | '(' | ')' | '.' => break,
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
                    Expr::Lambda(_arg, body) => body.beta_reduce(&evaluated_argument, 1).evaluate(), // We start from 1 (see above)
                    _ => self.clone(),
                }
            }
            expr => expr.clone(),
        };
        let new_str = format!("{}", result);
        if previous_str != new_str {
            println!("Evaluated: {} => {}", self, result);
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
            _ => todo!(),
        }
    }
}

fn main() {
    println!("TRUE: {}", Expr::TRUE());
    println!("FALSE: {}", Expr::FALSE());

    let test = Expr::from_str("( ( @x.@y.(x z)  @y.y ) wtf)");
    println!("{}", test);
    println!("{}", test.evaluate());

    let e = Expr::if_then_else(Expr::TRUE(), Expr::TRUE(), Expr::FALSE());
    println!("{e}");
    println!("{}", e.evaluate());

    let t2 = Expr::from_str("((and true) false)")
        .provide_variable("true", Expr::TRUE())
        .provide_variable("false", Expr::FALSE())
        .provide_variable("and", Expr::from_str("λp.λq.((p q) p)"));
    println!("{}", t2.evaluate());

    let y_combinator = Expr::from_str("@f.( (@x.(f (x x))) (@x.(f (x x))) )");
    println!("{}", y_combinator.evaluate());
}
