use std::fmt::Debug;

use crate::evaluator::{
    builtins::{arithmetic::ArithmeticTag, helpers::HelperFunctionTag, io::IOTag},
    reduction::ClosurePath,
    Graph,
};

pub mod arithmetic;
pub mod helpers;
pub mod io;

#[derive(Debug, Clone, Copy)]
pub enum ConstructorTag {
    IO(IOTag),
    Arithmetic(ArithmeticTag),
    HelperFunction(HelperFunctionTag),
    CustomTag { uid: usize, arity: usize },
}

impl ConstructorTag {
    pub fn from_str(str: &str) -> Option<Self> {
        match str {
            "#match" => Some(Self::HelperFunction(HelperFunctionTag::Match)),
            "#constructor" => Some(Self::HelperFunction(HelperFunctionTag::CreateConstructor)),

            "#io_getchar" => Some(Self::IO(IOTag::GetChar)),
            "#io_putchar" => Some(Self::IO(IOTag::PutChar)),
            "#io_throw" => Some(Self::IO(IOTag::Throw)),
            "#io_flatmap" => Some(Self::IO(IOTag::Flatmap)),

            "=num" => Some(Self::Arithmetic(ArithmeticTag::Eq)),
            "+" => Some(Self::Arithmetic(ArithmeticTag::Add)),
            "-" => Some(Self::Arithmetic(ArithmeticTag::Sub)),
            "*" => Some(Self::Arithmetic(ArithmeticTag::Mul)),
            "/" => Some(Self::Arithmetic(ArithmeticTag::Div)),
            "^" => Some(Self::Arithmetic(ArithmeticTag::Pow)),

            _ => None,
        }
    }
    pub fn argument_names(&self) -> Vec<&str> {
        match self {
            Self::IO(tag) => tag.argument_names(),
            Self::Arithmetic(tag) => tag.argument_names(),
            Self::HelperFunction(tag) => tag.argument_names(),
            Self::CustomTag { arity, .. } => {
                vec!["param"; *arity]
            }
        }
    }

    pub fn arity(&self) -> usize {
        self.argument_names().len()
    }

    pub fn is_value(&self) -> bool {
        match self {
            // It's cheaper to compute arithmetic immediately
            // than to carry around lazy data.
            // Flatmap is the only non-lazy IO function
            Self::Arithmetic(_) | Self::HelperFunction(_) | Self::IO(IOTag::Flatmap) => false,
            _ => true,
        }
    }
    pub fn evaluate(
        &self,
        graph: &mut Graph,
        closure_path: &mut ClosurePath,
        arguments: Vec<usize>,
    ) -> usize {
        match self {
            Self::Arithmetic(tag) => tag.evaluate(graph, closure_path, arguments),
            Self::HelperFunction(tag) => tag.evaluate(graph, closure_path, arguments),
            Self::IO(IOTag::Flatmap) => IOTag::flatmap(graph, closure_path, arguments),
            tag if tag.is_value() => panic!("This constructor is already a value"),
            _ => unreachable!(),
        }
    }
}
