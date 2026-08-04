use std::{cell::RefCell, rc::Rc};

use crate::{
    error::Error,
    expr::{Expr, SpannedExpr},
    spanned::Spanned,
};

pub type ValueIter = Rc<RefCell<dyn Iterator<Item = Value>>>;

/// Argument types recognized by `#[kaon::module]`
pub type Str = String;
pub type Int = i32;
pub type Float = f32;
pub type Bool = bool;
pub type List = Vec<Value>;
pub type Method = (Vec<String>, SpannedExpr);

/// The kind of a [`Value`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Null,
    Bool,
    Int,
    Float,
    Str,
    List,
    Iter,
    Method,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Type::Null => "null",
            Type::Bool => "bool",
            Type::Int => "int",
            Type::Float => "float",
            Type::Str => "str",
            Type::List => "list",
            Type::Iter => "iter",
            Type::Method => "method",
        };

        write!(f, "{name}")
    }
}

/// A value representable by kaon
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
    /// This value's type
    pub fn type_of(&self) -> Type {
        match self {
            Value::Null => Type::Null,
            Value::Bool(_) => Type::Bool,
            Value::Int(_) => Type::Int,
            Value::Float(_) => Type::Float,
            Value::Str(_) => Type::Str,
            Value::List(_) => Type::List,
            Value::Iter(_) => Type::Iter,
            Value::Method { .. } => Type::Method,
        }
    }

    /// Whether this value is null
    pub fn null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Attempts to downcast the value into a boolean
    pub fn bool(&self) -> Result<bool, Error> {
        match self {
            Value::Bool(b) => Ok(*b),
            other => Err(Error::ExpectedType(Type::Bool, other.type_of())),
        }
    }

    /// Attempts to downcast the value into an integer
    pub fn int(&self) -> Result<i32, Error> {
        match self {
            Value::Int(i) => Ok(*i),
            other => Err(Error::ExpectedType(Type::Int, other.type_of())),
        }
    }

    /// Attempts to downcast the value into a float
    pub fn float(&self) -> Result<f32, Error> {
        match self {
            Value::Float(x) => Ok(*x),
            other => Err(Error::ExpectedType(Type::Float, other.type_of())),
        }
    }

    /// Attepts to downcast the value into a str
    pub fn str(&self) -> Result<&str, Error> {
        match self {
            Value::Str(s) => Ok(s.as_str()),
            other => Err(Error::ExpectedType(Type::Str, other.type_of())),
        }
    }

    /// Checks the value is a method, and returns it unchanged
    pub fn method(self) -> Result<(Vec<String>, Spanned<Box<Expr>>), Error> {
        match self {
            Value::Method { args, body } => Ok((args, body)),
            other => Err(Error::ExpectedType(Type::Method, other.type_of())),
        }
    }

    /// Attempts to downcast the value into a list of generic items.
    pub fn list(&self) -> Result<Vec<Value>, Error> {
        match self {
            Value::List(items) => Ok(items.clone()),
            other => Err(Error::ExpectedType(Type::List, other.type_of())),
        }
    }

    /// Starts by downcasting the value into a list, then applies the given function to every element in the list
    pub fn mapped_list<T>(
        &self,
        mapper: impl Fn(&Value) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        self.list()?.iter().map(mapper).collect()
    }

    /// Turns the value into an iterator over kaon values
    pub fn into_values(self) -> Result<Box<dyn Iterator<Item = Value>>, Error> {
        match self {
            Value::List(items) => Ok(Box::new(items.into_iter())),
            Value::Iter(iter) => Ok(Box::new(std::iter::from_fn(move || {
                iter.borrow_mut().next()
            }))),
            Value::Str(s) => Ok(Box::new(
                s.chars()
                    .collect::<Vec<char>>()
                    .into_iter()
                    .map(|x| Value::Str(x.to_string())),
            )),
            other => Err(Error::NotIterable(other)),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Value::Iter(_) => write!(f, "<iter>"),
            Value::Method { .. } => write!(f, "<method>"),
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
            (Value::Method { args: a1, body: b1 }, Value::Method { args: a2, body: b2 }) => {
                a1 == a2 && b1 == b2
            }
            _ => false,
        }
    }
}
