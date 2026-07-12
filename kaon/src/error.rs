/// A reason lexing, parsing or executing failed
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("Expected '\"' to close the string, but found EOF")]
    UnclosedString,

    #[error("Unknown char: '{0}'")]
    UnknownChar(char),

    #[error("Unknown escape character in string: '{0}'")]
    UnknownEscape(char),
}
