use std::{collections::HashMap, path::PathBuf};

use kwark_text_buffer::{self as text, BoundCursorSet};
use kwark_text_buffer_renderer::Pipeline;

/// Some kind of text that can be rendered/used as a buffer
pub enum Buffer {
    Text {
        path: PathBuf,
        buf: text::BufferEntry,
        cursors: text::CursorSet,
    },
    Widget(ratatui::buffer::Buffer),
}

impl Buffer {
    /// Creates a new index for the Buffer given an incrementable index
    pub fn create_index(&self, tracker: &mut usize) -> ID {
        match self {
            Self::Text { path, .. } => ID::Text(path.clone()),
            Self::Widget(_) => {
                let id = ID::Widget(*tracker);
                *tracker += 1;
                id
            }
        }
    }

    /// Given the window, an area, and a pipeline, render the widget to that location
    pub fn render<S>(
        &self,
        pipeline: &mut Pipeline<S>,
        state: &mut S,
        frame: &mut ratatui::Frame<'_>,
        rect: ratatui::layout::Rect,
    ) {
        match self {
            Self::Text { buf, cursors, .. } => {
                let carets = cursors.cursors();
                let primary = cursors.primary();
                pipeline.render(
                    buf.buffer.rope(),
                    state,
                    0,
                    carets,
                    primary,
                    rect,
                    frame.buffer_mut(),
                );
            }
            Self::Widget(_) => {}
        }
    }

    /// Attempts to turn the buffer into a text-buffer bound to the cursor
    ///
    /// Returns `None` if the buffer isn't a text-buffer
    pub fn as_text<'a>(&'a mut self) -> Option<BoundCursorSet<'a>> {
        match self {
            Self::Text { buf, cursors, .. } => Some(cursors.bind(&mut buf.buffer)),
            Self::Widget(_) => None,
        }
    }
}

/// A basic Identifier to differentiate between different buffers
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ID {
    Text(PathBuf),
    Widget(usize),
}

/// A basic storage for a list of Buffers
#[derive(Default)]
pub struct Storage {
    next_id: usize,
    inner: HashMap<ID, Buffer>,
}

impl Storage {
    /// Iterate through all of the active buffers
    pub fn iter(&self) -> impl Iterator<Item = (&ID, &Buffer)> {
        self.inner.iter()
    }

    /// Iterate through all of the active buffers
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ID, &mut Buffer)> {
        self.inner.iter_mut()
    }

    /// Retrieves a buffer with the given ID from storage,
    /// Returns [`None`] if the ID isn't stored.
    ///
    /// For an immutable version, see [`Self::get_mut`]
    pub fn get_mut(&mut self, id: &ID) -> Option<&mut Buffer> {
        self.inner.get_mut(id)
    }

    /// Retrieves a buffer with the given ID from storage,
    /// Returns [`None`] if the ID isn't stored.
    ///
    /// For a mutable version, see [`Self::get_mut`]
    pub fn get(&self, id: &ID) -> Option<&Buffer> {
        self.inner.get(id)
    }

    /// Retrieves the buffer with the given ID, or inserts a new Buffer created with a new ID
    ///
    /// Returns an ([`ID`], [`Buffer`]) pair
    pub fn get_or_insert(&mut self, id: ID, creator: impl FnOnce() -> Buffer) -> (ID, &mut Buffer) {
        if self.inner.contains_key(&id) {
            let buf = self
                .get_mut(&id)
                .expect("Value was just checked for existance");

            return (id, buf);
        }

        let buf = creator();
        let id = buf.create_index(&mut self.next_id);

        self.inner.insert(id.clone(), buf);

        let buf = self
            .inner
            .get_mut(&id)
            .expect("Value was just inserted into the buffer");

        (id, buf)
    }

    /// Creates a new buffer with the given ID, trashes the old buffer if the ID already existed
    pub fn insert(&mut self, buf: Buffer) -> ID {
        let id = buf.create_index(&mut self.next_id);

        self.inner.insert(id.clone(), buf);

        id
    }

    /// Checks if the ID exists within the storage
    pub fn has(&self, id: &ID) -> bool {
        self.inner.contains_key(id)
    }
}

/// A storage of [`ID`]s with an actively selected buffer
pub struct List {
    active: usize,
    ids: Vec<ID>,
}

impl List {
    /// Sets the actively selected buffer to the passed [`ID`]
    ///
    /// If not already in the list, inserts it
    pub fn select(&mut self, id: ID) {
        if let Some((i, _)) = self.ids.iter().enumerate().find(|(_, x)| **x == id) {
            self.active = i;
            return;
        }

        self.active = self.ids.len();
        self.ids.push(id);
    }

    /// Returns the actively selected [`ID`] for the List
    ///
    /// Returns [`None`] if no buffer was selected
    pub fn selected(&self) -> Option<ID> {
        if self.ids.is_empty() {
            return None;
        }

        Some(self.ids[self.active].clone())
    }

    /// Closes the currently selected buffer
    pub fn close_primary(&mut self) {
        if self.ids.is_empty() {
            return;
        }

        self.ids.remove(self.active);
        self.active = self.active.saturating_sub(1);
    }

    /// Closes all buffers other than the currently selected one
    pub fn close_other(&mut self) {
        let id = self.ids.remove(self.active);
        self.ids.clear();

        self.active = 0;

        self.ids.push(id);
    }

    /// Moves the primary selection the given distance
    ///
    /// Wraps if going `< 0` or `> list length`
    pub fn move_primary(&mut self, dist: isize) {
        let optional = self.active as isize + dist;
        self.active = optional.rem_euclid(self.ids.len() as isize) as usize;
    }

    /// Remove the id from the list
    pub fn remove(&mut self, id: ID) {
        self.ids.retain(|x| *x != id);
    }

    /// Updates the List to match the Storage's stored IDs
    pub fn update(&mut self, storage: &Storage) {
        self.ids.retain(|x| storage.has(x));

        self.active = self.active.min(self.ids.len() - 1);
    }
}
