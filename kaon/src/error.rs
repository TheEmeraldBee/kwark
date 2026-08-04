use crate::value::{Type, Value};

/// A reason lexing, parsing or executing failed
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("Expected '\"' to close the string, but found EOF")]
    UnclosedString,

    #[error("Expected '*/' to close the comment, but found EOF")]
    UnclosedComment,

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

    #[error("Unknown argument: '{0}'")]
    UnknownArg(String),

    #[error("Value is not callable: {0:?}")]
    NotCallable(Value),

    #[error("Expected {expected} arguments, found {got}")]
    WrongArgCount { expected: usize, got: usize },

    #[error("Expected a value of type {0}, found a {1}")]
    ExpectedType(Type, Type),

    #[error("Value is not iterable: {0:?}")]
    NotIterable(Value),

    #[error("Type mismatch: {0}")]
    TypeMismatch(String),

    #[error("{0}")]
    External(String),
}

pub trait IntoKaonError<T> {
    fn into_kaon(self) -> Result<T, Error>;
}

impl<T, E: std::error::Error> IntoKaonError<T> for Result<T, E> {
    fn into_kaon(self) -> Result<T, Error> {
        self.map_err(|e| Error::External(e.to_string()))
    }
}
