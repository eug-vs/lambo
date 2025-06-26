use std::fmt::Display;

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
        let result = Self::from_chars(&mut s.chars());
        assert_eq!(format!("{}", result), s);
        result
    }
    fn from_chars<I: Iterator<Item = char>>(iterator: &mut I) -> Self {
        match iterator.next().unwrap() {
            '(' => {
                let func = Self::from_chars(iterator);
                let space = iterator.next().unwrap();
                assert_eq!(space, ' ');
                let arg = Self::from_chars(iterator);
                let paren = iterator.next().unwrap();
                assert_eq!(paren, ')');
                Expr::Call(Box::new(func), Box::new(arg))
            }
            'λ' => {
                let var = iterator.next().unwrap();
                let dot = iterator.next().unwrap();
                assert_eq!(dot, '.');
                let body = Self::from_chars(iterator);
                Expr::Lambda(String::from(var), Box::new(body))
            }
            c => Expr::Var(String::from(c)),
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

    let t = Expr::from_str("((λx.λy.x a) b)");
    println!("{}", t.evaluate());

    let t2 = Expr::from_str("(λt.((t a) b) λx.λy.x)");
    println!("{}", t2.evaluate());
}
