use std::{fmt::Display, path::PathBuf};

mod buffer;
pub use buffer::*;

mod error;
pub use error::*;

mod operation;
use normpath::PathExt;
pub(crate) use operation::*;

mod cursor;
pub use cursor::*;

pub struct BufferEntry {
    pub buffer: Buffer,
    kind: BufferKind,
}

impl BufferEntry {
    pub fn new_scratch(name: String) -> Self {
        Self {
            buffer: Buffer::default(),
            kind: BufferKind::Scratch(name),
        }
    }

    pub fn new_file(filepath: PathBuf) -> Result<Self> {
        let canon_path = filepath.normalize()?;

        let file = std::fs::File::open(&canon_path)?;

        let buf = Buffer::from_reader(file)?;

        Ok(Self {
            buffer: buf,
            kind: BufferKind::File(filepath),
        })
    }

    /// Retrieves the kind of the buffer from it's entry
    pub fn kind(&self) -> &BufferKind {
        &self.kind
    }
}

#[derive(PartialEq, Eq, Hash)]
pub enum BufferKind {
    Scratch(String),
    File(PathBuf),
}

impl Display for BufferKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scratch(name) => write!(f, "{name}"),
            Self::File(path) => write!(f, "{}", path.display()),
        }
    }
}
