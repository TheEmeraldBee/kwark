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
        self.save_job(path).run()
    }

    /// Returns a save job for the current text
    pub fn save_job(&self, path: Option<PathBuf>) -> SaveJob {
        SaveJob {
            rope: self.buffer.rope().clone(),
            kind: self.kind.clone(),
            path,
        }
    }

    /// Retrieves the kind of the buffer from it's entry
    pub fn kind(&self) -> &BufferKind {
        &self.kind
    }
}

/// A snapshot of a buffer's text and where to save it
pub struct SaveJob {
    rope: ropey::Rope,
    kind: BufferKind,
    path: Option<PathBuf>,
}

impl SaveJob {
    /// Writes the snapshot to disk
    pub fn run(self) -> Result<()> {
        let writer = match self.kind {
            BufferKind::Scratch(n) => std::fs::File::create_new(self.path.unwrap_or(n.into()))?,
            BufferKind::File(p) => std::fs::File::create(self.path.unwrap_or(p))?,
        };

        self.rope.write_to(writer)?;

        Ok(())
    }

    /// The display name of the buffer being saved
    pub fn name(&self) -> String {
        self.kind.to_string()
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
