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
    pub fn new_scratch(name: impl Into<String>) -> Self {
        Self {
            buffer: Buffer::default(),
            kind: BufferKind::Scratch(name.into()),
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

    pub fn save(&self, path: Option<PathBuf>) -> Result<()> {
        let writer = match self.kind.clone() {
            BufferKind::Scratch(n) => std::fs::File::create_new(path.unwrap_or(n.into()))?,
            BufferKind::File(p) => std::fs::File::create(path.unwrap_or(p))?,
        };

        self.buffer.write(writer)?;

        Ok(())
    }

    /// Retrieves the kind of the buffer from it's entry
    pub fn kind(&self) -> &BufferKind {
        &self.kind
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum BufferKind {
    Scratch(String),
    File(PathBuf),
}

impl Display for BufferKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scratch(name) => write!(f, "{name}"),
            Self::File(path) => write!(
                f,
                "{}",
                path.file_name()
                    .map(|x| x.display().to_string())
                    .unwrap_or_default()
            ),
        }
    }
}
