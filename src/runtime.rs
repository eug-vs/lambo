use std::io::{Write, stdin, stdout};

use crate::{Expr, Variable};

pub enum IOMonad {
    /// An arbitrary expression lifted into IO
    Pure(Expr),
    /// Some expression to-be-printed to console
    Print(Expr),
    /// Some expression to-be-thrown
    Throw(Expr),
    /// An action that will read and parse lambda from STDIN
    ReadLambda,
    /// A chain of IO operations
    Flatmap(Box<IOMonad>, Expr),
}

impl IOMonad {
    pub fn from_expr(expr: &Expr) -> Self {
        match expr {
            Expr::Var(Variable { name, kind: _ }) if name == "#io_read" => {
                return IOMonad::ReadLambda;
            }
            Expr::Call(func, argument) => match &**func {
                Expr::Var(Variable { name, kind: _ }) if name == "#io_pure" => {
                    return IOMonad::Pure(*argument.clone());
                }
                Expr::Var(Variable { name, kind: _ }) if name == "#io_print" => {
                    return IOMonad::Print(*argument.clone());
                }
                Expr::Var(Variable { name, kind: _ }) if name == "#io_throw" => {
                    return IOMonad::Throw(*argument.clone());
                }
                Expr::Call(inner_func, inner_arg) => match &**inner_func {
                    Expr::Var(Variable { name, kind: _ }) if name == "#io_flatmap" => {
                        let monad = IOMonad::from_expr(&inner_arg);
                        return IOMonad::Flatmap(Box::new(monad), *argument.clone());
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
        // println!("WARN: implicitly falling back to #pure IO monad");
        IOMonad::Pure(expr.clone())
    }

    pub fn unwrap(&self) -> Expr {
        match self {
            Self::Pure(expr) => expr.evaluate(),
            Self::Print(expr) => {
                println!("=>  {}", expr);
                expr.evaluate()
            }
            Self::Throw(expr) => {
                panic!("{}", expr);
            }
            Self::ReadLambda => {
                print!("$   ");
                stdout().flush().unwrap();
                let mut s = String::new();
                stdin().read_line(&mut s).unwrap();
                Expr::from_str(&s).evaluate()
            }
            Self::Flatmap(io, transform) => {
                let unwrapped = io.unwrap();
                let transform_result =
                    Expr::Call(Box::new(transform.clone()), Box::new(unwrapped)).evaluate();
                IOMonad::from_expr(&transform_result).unwrap()
            }
        }
    }
}
