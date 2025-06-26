use std::{fmt::Display, iter::Peekable};

#[derive(Debug, Clone)]
enum Expr {
    Var(String),
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
        Self::Call(
            Box::new(Self::Lambda(
                String::from(variable_name),
                Box::new(self.clone()),
            )),
            Box::new(value),
        )
    }
    fn from_chars<I: Iterator<Item = char>>(iterator: &mut Peekable<I>) -> Self {
        Self::consume_whitespace(iterator);
        match iterator.peek().unwrap() {
            '(' => {
                iterator.next().unwrap();

                Self::consume_whitespace(iterator);

                let func = Self::from_chars(iterator);

                Self::consume_whitespace(iterator);

                let arg = Self::from_chars(iterator);

                Self::consume_whitespace(iterator);

                let paren = iterator.next().unwrap();
                assert_eq!(paren, ')');
                Expr::Call(Box::new(func), Box::new(arg))
            }
            'λ' | '@' => {
                iterator.next().unwrap();
                let var = Self::variable_name_from_chars(iterator);
                let dot = iterator.next().unwrap();
                assert_eq!(dot, '.');
                let body = Self::from_chars(iterator);
                Expr::Lambda(var, Box::new(body))
            }
            _ => Expr::Var(Self::variable_name_from_chars(iterator)),
        }
    }
    fn evaluate(&self) -> Self {
        println!("Evaluate     : {}", self);
        let result = match self {
            Expr::Call(function, argument) => match (**function).evaluate() {
                Expr::Lambda(arg, body) => body.beta_reduce(arg, argument).evaluate(),
                _ => self.clone(),
            },
            expr => expr.clone(),
        };
        println!("Evaluate done: {} => {}", self, result);
        result
    }
    fn beta_reduce(&self, variable: String, replace_with: &Expr) -> Expr {
        match self {
            Expr::Var(var) => {
                if *var == variable {
                    replace_with.clone()
                } else {
                    Expr::Var(var.clone())
                }
            }
            Expr::Lambda(arg, body) => {
                assert_ne!(
                    *arg, variable,
                    "Variable name clash! Variable: {}, expression: {}",
                    variable, self
                );
                Expr::Lambda(
                    arg.clone(),
                    Box::new(body.beta_reduce(variable, replace_with)),
                )
            }
            Expr::Call(func, arg) => Expr::Call(
                Box::new(func.beta_reduce(variable.clone(), replace_with)),
                Box::new(arg.beta_reduce(variable, replace_with)),
            ),
        }
    }
}

fn main() {
    let e = Expr::from_str("(λx.y a)");
    println!("{}", e.evaluate());

    let t2 = Expr::from_str("((and true) false)")
        .provide_variable("true", Expr::from_str("λx.λy.x"))
        .provide_variable("false", Expr::from_str("λa.λb.b"))
        .provide_variable("and", Expr::from_str("λp.λq.((p q) p)"));
    println!("{}", t2.evaluate());
}
