use std::path::PathBuf;

use tree_sitter::{IncludedRangesError, QueryError};

/// The set of errors the parsing engine can return
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Language {0} doesn't exist in the store")]
    NoLanguage(String),

    #[error("Parser failed to produce a tree")]
    ParseFailed,

    #[error(transparent)]
    IncludedRanges(#[from] IncludedRangesError),

    #[error(transparent)]
    QueryLoad(#[from] QueryLoadError),
}

/// The set of errors that attempting to load a query can return
#[derive(thiserror::Error, Debug)]
pub enum QueryLoadError {
    #[error("Language {0} doesn't exist in the store")]
    NoLanguage(String),

    #[error(
        "Query path '{0}' doesn't exist in the world, make sure your folders are layed out correctly"
    )]
    MissingPath(PathBuf),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    QueryError(#[from] QueryError),
}
