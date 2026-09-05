use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IOError(#[from] io::Error),

    #[error("Version given is out of date with current buffer version")]
    OutdatedVersion,

    #[error(transparent)]
    RopeyError(#[from] ropey::Error),
}

pub type Result<T> = core::result::Result<T, Error>;
