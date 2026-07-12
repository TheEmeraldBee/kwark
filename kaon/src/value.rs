use std::{cell::RefCell, rc::Rc};

use crate::{error::Error, expr::SpannedExpr};

pub type ValueIter = Rc<RefCell<dyn Iterator<Item = Value>>>;

#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i32),
    Float(f32),
    Str(String),

    List(Vec<Value>),
    Iter(ValueIter),

    Method {
        args: Vec<String>,
        body: SpannedExpr,
    },
}

impl Value {
    /// The value as a bool, used for if/while conditions
    pub fn truthy(&self) -> Result<bool, Error> {
        match self {
            Value::Bool(b) => Ok(*b),
            other => Err(Error::NotABool(other.clone())),
        }
    }

    /// Turns the value into a rust iterator over values, used for `for x in ...`
    pub fn into_values(self) -> Result<Box<dyn Iterator<Item = Value>>, Error> {
        match self {
            Value::List(items) => Ok(Box::new(items.into_iter())),
            Value::Iter(iter) => Ok(Box::new(std::iter::from_fn(move || iter.borrow_mut().next()))),
            other => Err(Error::NotIterable(other)),
        }
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "Null"),
            Value::Bool(b) => write!(f, "Bool({b})"),
            Value::Int(i) => write!(f, "Int({i})"),
            Value::Float(x) => write!(f, "Float({x})"),
            Value::Str(s) => write!(f, "Str({s:?})"),
            Value::List(items) => write!(f, "List({items:?})"),
            Value::Iter(_) => write!(f, "Iter(..)"),
            Value::Method { args, .. } => write!(f, "Method({args:?})"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Iter(a), Value::Iter(b)) => Rc::ptr_eq(a, b),
            (
                Value::Method {
                    args: a1,
                    body: b1,
                },
                Value::Method {
                    args: a2,
                    body: b2,
                },
            ) => a1 == a2 && b1 == b2,
            _ => false,
        }
    }
}

//TODO: Implement functions to auto-convert to values without having to match
