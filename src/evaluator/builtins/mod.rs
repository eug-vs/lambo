use std::fmt::Debug;
use std::{collections::HashMap, rc::Rc};

use crate::evaluator::Graph;

pub mod arithmetic;
pub mod io;

pub struct BuiltinFunctionDeclaration {
    pub name: String,
    /// How many arguments this function takes
    pub argument_names: Vec<String>,
    /// ()
    pub to_value: Box<dyn Fn(&mut Graph, Vec<usize>) -> usize>,
}

impl BuiltinFunctionDeclaration {
    pub fn arity(&self) -> usize {
        self.argument_names.len()
    }
}

impl Debug for BuiltinFunctionDeclaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "builtin {}-ary function {}", self.arity(), self.name)
    }
}

pub type BuiltinFunctionRegistry = HashMap<String, Rc<BuiltinFunctionDeclaration>>;
