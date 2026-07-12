use crate::value::Value;

/// A reason lexing, parsing or executing failed
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("Expected '\"' to close the string, but found EOF")]
    UnclosedString,

    #[error("Unknown char: '{0}'")]
    UnknownChar(char),

    #[error("Unknown escape character in string: '{0}'")]
    UnknownEscape(char),

    #[error("Expected {0}, found {1}")]
    Expected(String, String),

    #[error("Expected {0}, found EOF")]
    ExpectedFoundEOF(String),

    #[error("Unknown local: '{0}'")]
    UnknownLocal(String),

    #[error("Unknown operator: '{0}'")]
    UnknownOperator(String),

    #[error("Unknown function: '{0}'")]
    UnknownFunction(String),

    #[error("Value is not callable: {0:?}")]
    NotCallable(Value),

    #[error("Expected {expected} arguments, found {got}")]
    WrongArgCount { expected: usize, got: usize },

    #[error("Expected a bool, found {0:?}")]
    NotABool(Value),

    #[error("Value is not iterable: {0:?}")]
    NotIterable(Value),

    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
}
