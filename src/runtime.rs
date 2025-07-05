use std::io::{Write, stdin, stdout};

use crate::Expr;

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
            Expr::Var { name, .. } if name == "#io_read" => {
                return Some(Ok(IOMonad::ReadLambda));
            }
            Expr::Call {
                function,
                parameter,
                ..
            } => match &**function {
                Expr::Var { name, .. } if name == "#io_pure" => {
                    return Some(Ok(IOMonad::Pure(*parameter.clone())));
                }
                Expr::Var { name, .. } if name == "#io_print" => {
                    return Some(Ok(IOMonad::Print(*parameter.clone())));
                }
                Expr::Var { name, .. } if name == "#io_dbg" => {
                    return Some(Ok(IOMonad::Debug(*parameter.clone())));
                }
                Expr::Var { name, .. } if name == "#io_throw" => {
                    return Some(Ok(IOMonad::Throw(*parameter.clone())));
                }
                Expr::Call {
                    function: inner_function,
                    parameter: inner_parameter,
                    ..
                } => match &**inner_function {
                    Expr::Var { name, .. } if name == "#io_flatmap" => {
                        return IOMonad::from_expr(&inner_parameter).map_or(
                            Some(Err(format!(
                                "Arguments to #io_flatmap must be IO, got: {}",
                                inner_parameter
                            ))),
                            |result| {
                                Some(result.map(|monad| {
                                    IOMonad::Flatmap(Box::new(monad), *inner_parameter.clone())
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
                expr.evaluate();
                Ok(expr.clone())
            }
            Self::Print(expr) => {
                expr.evaluate();
                println!("=>  {}", expr);
                Ok(expr.clone())
            }
            Self::Debug(expr) => {
                expr.evaluate();
                println!("=>  {}", expr.fmt_de_brujin());
                Ok(expr.clone())
            }
            Self::Throw(expr) => {
                expr.evaluate();
                panic!("{}", expr);
            }
            Self::ReadLambda => {
                print!("$   ");
                stdout().flush().unwrap();
                let mut s = String::new();
                stdin().read_line(&mut s).unwrap();
                let mut expr = Expr::from_str(&s);
                expr.evaluate();
                Ok(expr)
            }
            Self::Flatmap(io, transform) => {
                let unwrapped = io.unwrap()?;
                let mut transform_result = Expr::Call {
                    function: Box::new(transform.clone()),
                    parameter: Box::new(unwrapped),
                };
                transform_result.evaluate();
                IOMonad::from_expr(&transform_result)
                    .unwrap_or(Err(format!(
                        "Evaluated result is not an IO (this will be later checked at type level): {transform_result}"
                    )))?
                    .unwrap()
            }
        }
    }
}
