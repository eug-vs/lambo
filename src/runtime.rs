use std::io::{Write, stdin, stdout};

use crate::{Expr, Variable};

pub enum IOMonad {
    /// An arbitrary expression lifted into IO
    Pure(Expr),
    /// Some expression to-be-printed to console
    Print(Expr),
    Debug(Expr),
    /// Some expression to-be-thrown
    Throw(Expr),
    /// An action that will read and parse lambda from STDIN
    ReadLambda,
    /// A chain of IO operations
    Flatmap(Box<IOMonad>, Expr),
}

impl IOMonad {
    /// Return type is Option<Result> because expression may not be a IO at all.
    /// But if it is, it has to match IO semantics
    pub fn from_expr(expr: &Expr) -> Option<Result<Self, String>> {
        match expr {
            Expr::Var(Variable { name, kind: _ }) if name == "#io_read" => {
                return Some(Ok(IOMonad::ReadLambda));
            }
            Expr::Call(func, argument) => match &**func {
                Expr::Var(Variable { name, kind: _ }) if name == "#io_pure" => {
                    return Some(Ok(IOMonad::Pure(*argument.clone())));
                }
                Expr::Var(Variable { name, kind: _ }) if name == "#io_print" => {
                    return Some(Ok(IOMonad::Print(*argument.clone())));
                }
                Expr::Var(Variable { name, kind: _ }) if name == "#io_dbg" => {
                    return Some(Ok(IOMonad::Debug(*argument.clone())));
                }
                Expr::Var(Variable { name, kind: _ }) if name == "#io_throw" => {
                    return Some(Ok(IOMonad::Throw(*argument.clone())));
                }
                Expr::Call(inner_func, inner_arg) => match &**inner_func {
                    Expr::Var(Variable { name, kind: _ }) if name == "#io_flatmap" => {
                        return IOMonad::from_expr(&inner_arg).map_or(
                            Some(Err(format!(
                                "Arguments to #io_flatmap must be IO, got: {}",
                                inner_arg
                            ))),
                            |result| {
                                Some(result.map(|monad| {
                                    IOMonad::Flatmap(Box::new(monad), *argument.clone())
                                }))
                            },
                        );
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
        None
    }

    /// Flatmapping can fail if provided transform function does not return IO
    pub fn unwrap(&mut self) -> Result<Expr, String> {
        match self {
            Self::Pure(expr) => {
                expr.evaluate_lazy();
                Ok(expr.clone())
            }
            Self::Print(expr) => {
                expr.evaluate_lazy();
                println!("=>  {}", expr);
                Ok(expr.clone())
            }
            Self::Debug(expr) => {
                expr.evaluate_lazy();
                println!("=>  {}", expr.fmt_de_brujin());
                Ok(expr.clone())
            }
            Self::Throw(expr) => {
                expr.evaluate_lazy();
                panic!("{}", expr);
            }
            Self::ReadLambda => {
                print!("$   ");
                stdout().flush().unwrap();
                let mut s = String::new();
                stdin().read_line(&mut s).unwrap();
                let mut expr = Expr::from_str(&s);
                expr.evaluate_lazy();
                Ok(expr)
            }
            Self::Flatmap(io, transform) => {
                let unwrapped = io.unwrap()?;
                let mut transform_result =
                    Expr::Call(Box::new(transform.clone()), Box::new(unwrapped));
                transform_result.evaluate_lazy();
                IOMonad::from_expr(&transform_result)
                    .unwrap_or(Err(format!(
                        "Evaluated result is not an IO (this will be later checked at type level): {transform_result}"
                    )))?
                    .unwrap()
            }
        }
    }
}
