use std::{collections::HashMap, fmt::Display, path::PathBuf};

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

/// An index that can index into a buffer-list
pub type BufferID = u32;

/// A list of buffers to handle storage and data for the buffers
#[derive(Default)]
pub struct BufferList {
    buffers: HashMap<BufferID, BufferEntry>,
    by_kind: HashMap<BufferKind, BufferID>,
    next_id: BufferID,
}

impl BufferList {
    /// Create a new buffer by loading the file into memory
    ///
    /// If the file is already in memory, fetches it for you.
    pub fn file(&mut self, path: impl Into<PathBuf>) -> Result<BufferID> {
        let path = path.into();
        let kind = BufferKind::File(path.clone());

        if let Some(id) = self.by_kind.get(&kind) {
            return Ok(*id);
        }

        let id = self.next_id;

        let buf = BufferEntry::new_file(path)?;

        self.buffers.insert(id, buf);
        self.by_kind.insert(kind, id);

        self.next_id += 1;

        Ok(id)
    }

    /// Create a new buffer with the given name, initializing it as empty
    pub fn scratch(&mut self, name: String) -> BufferID {
        let kind = BufferKind::Scratch(name.clone());

        if let Some(id) = self.by_kind.get(&kind) {
            return *id;
        }

        let id = self.next_id;

        let buf = BufferEntry::new_scratch(name);

        self.buffers.insert(id, buf);
        self.by_kind.insert(kind, id);

        self.next_id += 1;

        id
    }

    /// Remove a buffer by id from the list
    pub fn delete(&mut self, id: BufferID) -> bool {
        let Some(buf) = self.buffers.remove(&id) else {
            return false;
        };

        self.by_kind.remove(&buf.kind);

        true
    }

    pub fn iter(&self) -> impl Iterator<Item = (&BufferID, &BufferEntry)> {
        self.buffers.iter()
    }

    /// Retrieves the buffer by ID
    pub fn get(&mut self, id: BufferID) -> Option<&mut Buffer> {
        let buf = self.buffers.get_mut(&id)?;

        Some(&mut buf.buffer)
    }

    pub fn get_raw(&mut self, id: BufferID) -> Option<&mut BufferEntry> {
        let buf = self.buffers.get_mut(&id)?;

        Some(buf)
    }
}
