use std::{collections::HashMap, fmt::Display, path::PathBuf};

use kwark_text::{self as text, BoundCursorSet, BufferKind};
use kwark_render::{HitArgs, Pipeline, RenderArgs, Viewport};
use kwark_tree_sitter::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Buffer(#[from] kwark_text::Error),

    #[error(transparent)]
    TreeSitter(#[from] kwark_tree_sitter::error::Error),
}

pub type Result<T> = core::result::Result<T, Error>;

/// A line, column, and character offset found by a hit test
#[derive(Clone, Copy, Debug)]
pub struct HitPosition {
    pub line: usize,
    pub col: usize,
    pub chr: usize,
}

/// Some kind of text that can be rendered/used as a buffer
pub enum Buffer {
    Text {
        id: text::BufferKind,
        buf: Box<text::BufferEntry>,
        cursors: Box<text::CursorSet>,

        tree: Option<SyntaxTree>,

        /// The buffer version the tree was last parsed at
        parsed_version: u128,
    },
    Widget(ratatui::buffer::Buffer),
}

impl From<text::BufferEntry> for Buffer {
    fn from(value: text::BufferEntry) -> Self {
        Self::Text {
            id: value.kind().clone(),
            buf: Box::new(value),
            cursors: Box::new(text::CursorSet::new()),

            tree: None,

            parsed_version: 0,
        }
    }
}

impl Buffer {
    /// Creates a new index for the Buffer given an incrementable index
    pub fn create_index(&self, tracker: &mut usize) -> ID {
        match self {
            Self::Text { id, .. } => ID::Text(id.clone()),
            Self::Widget(_) => {
                let id = ID::Widget(*tracker);
                *tracker += 1;
                id
            }
        }
    }

    /// Reparses the tree if the buffer's text has changed since the last parse,
    /// or parses it for the first time if it has no tree yet
    pub fn update(&mut self, store: &mut LanguageStore) -> Result<()> {
        match self {
            Self::Text {
                id,
                buf,
                tree,
                parsed_version,
                ..
            } => {
                let Some(tree) = tree else {
                    let path = match id {
                        text::BufferKind::File(path) => Some(path.as_path()),
                        text::BufferKind::Scratch(_) => None,
                    };
                    let first_line = buf.buffer.rope().get_line(0).map(|l| l.to_string());

                    let Some(lang) = store.detect(path, first_line.as_deref()) else {
                        return Ok(());
                    };
                    let lang = lang.to_string();

                    *tree = Some(SyntaxTree::parse(store, &lang, buf.buffer.rope())?);
                    *parsed_version = buf.buffer.version();

                    return Ok(());
                };

                let version = buf.buffer.version();
                if *parsed_version == version {
                    return Ok(());
                }

                let edits = buf.buffer.edits_since(*parsed_version)?;
                tree.reparse(store, edits, buf.buffer.rope())?;
                *parsed_version = version;

                Ok(())
            }

            Self::Widget(_) => Ok(()),
        }
    }

    /// Given the window, an area, and a pipeline, render the widget to that location
    ///
    /// Returns the range of lines that were rendered
    pub fn render<S>(
        &self,
        pipeline: &mut Pipeline<S>,
        state: &mut S,
        frame: &mut ratatui::Frame<'_>,
        rect: ratatui::layout::Rect,
        scroll_top: usize,
    ) -> Viewport {
        match self {
            Self::Text {
                buf, cursors, tree, ..
            } => {
                let carets = cursors.cursors();
                let primary = cursors.primary();
                pipeline.render(RenderArgs {
                    rope: buf.buffer.rope(),
                    scroll_top,
                    cursors: carets,
                    tree: tree.as_ref(),
                    area: rect,
                    buf: frame.buffer_mut(),

                    state,
                    primary,
                })
            }
            Self::Widget(_) => Viewport::default(),
        }
    }

    /// Finds the line, column, and character offset under a screen position
    pub fn hit_test<S>(
        &self,
        pipeline: &mut Pipeline<S>,
        state: &mut S,
        rect: ratatui::layout::Rect,
        scroll_top: usize,
        point: ratatui::layout::Position,
    ) -> Option<HitPosition> {
        match self {
            Self::Text {
                buf, cursors, tree, ..
            } => {
                let carets = cursors.cursors();
                let primary = cursors.primary();

                let (line, col) = pipeline.hit_test(HitArgs {
                    rope: buf.buffer.rope(),
                    state,
                    scroll_top,
                    cursors: carets,
                    primary,
                    tree: tree.as_ref(),
                    area: rect,
                    point,
                })?;

                Some(HitPosition {
                    line,
                    col,
                    chr: buf.buffer.line_col_to_char(line, col),
                })
            }
            Self::Widget(_) => None,
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
    Text(kwark_text::BufferKind),
    Widget(usize),
}

impl ID {
    /// Creates a new ID for a scratch buffer with the given name
    pub fn scratch(name: impl Into<String>) -> Self {
        Self::Text(BufferKind::Scratch(name.into()))
    }

    /// Creates a new ID for a file buffer with the given name
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::Text(BufferKind::File(path.into()))
    }

    /// Creates a new ID for a widget buffer with the given name
    pub fn widget(id: usize) -> Self {
        Self::Widget(id)
    }
}

impl Display for ID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Text(t) => t.to_string(),
                Self::Widget(i) => i.to_string(),
            }
        )
    }
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
    pub fn get_or_insert<B: Into<Buffer>>(
        &mut self,
        id: ID,
        creator: impl FnOnce() -> B,
    ) -> (ID, &mut Buffer) {
        if self.inner.contains_key(&id) {
            let buf = self
                .get_mut(&id)
                .expect("Value was just checked for existance");

            return (id, buf);
        }

        let buf = creator().into();
        let id = buf.create_index(&mut self.next_id);

        self.inner.insert(id.clone(), buf);

        let buf = self
            .inner
            .get_mut(&id)
            .expect("Value was just inserted into the buffer");

        (id, buf)
    }

    /// Creates a new buffer with the given ID, trashes the old buffer if the ID already existed
    pub fn insert(&mut self, buf: impl Into<Buffer>) -> ID {
        let buf = buf.into();
        let id = buf.create_index(&mut self.next_id);

        self.inner.insert(id.clone(), buf);

        id
    }

    /// Checks if the ID exists within the storage
    pub fn has(&self, id: &ID) -> bool {
        self.inner.contains_key(id)
    }

    /// Updates every buffer, reparsing trees whose version has changed
    pub fn update_all(&mut self, store: &mut LanguageStore) -> Result<()> {
        for buf in self.inner.values_mut() {
            buf.update(store)?;
        }

        Ok(())
    }
}

/// A storage of [`ID`]s with an actively selected buffer, each carrying a per-entry state `S`
pub struct List<S = ()> {
    active: usize,
    entries: Vec<(ID, S)>,
}

impl<S> Default for List<S> {
    fn default() -> Self {
        Self {
            active: 0,
            entries: Vec::new(),
        }
    }
}

impl<S: Default> List<S> {
    /// Creates a list that gives every buffer slot a default `S` as it's added
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the actively selected buffer to the passed [`ID`]
    ///
    /// If not already in the list, inserts it with a default state
    pub fn select(&mut self, id: ID) {
        if let Some(i) = self.entries.iter().position(|(x, _)| *x == id) {
            self.active = i;
            return;
        }

        self.active = self.entries.len();
        self.entries.push((id, S::default()));
    }
}

impl<S> List<S> {
    /// Returns whether or not there is a buffer open in the list
    pub fn is_empty(&mut self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate through all buffers in the list, paired with a boolean of whether it's the selected
    pub fn iter(&self) -> impl Iterator<Item = (bool, &ID)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (self.active == i, id))
    }

    /// Returns the actively selected [`ID`] for the List
    ///
    /// Returns [`None`] if no buffer was selected
    pub fn selected(&self) -> Option<ID> {
        self.entries.get(self.active).map(|(id, _)| id.clone())
    }

    /// Returns the state belonging to the selected buffer
    pub fn state(&self) -> Option<&S> {
        self.entries.get(self.active).map(|(_, s)| s)
    }

    /// Returns the state belonging to the selected buffer, mutably
    pub fn state_mut(&mut self) -> Option<&mut S> {
        self.entries.get_mut(self.active).map(|(_, s)| s)
    }

    /// Returns the state belonging to a specific buffer, selected or not
    pub fn state_for_mut(&mut self, id: &ID) -> Option<&mut S> {
        self.entries
            .iter_mut()
            .find(|(x, _)| x == id)
            .map(|(_, s)| s)
    }

    /// Closes the currently selected buffer
    pub fn close_primary(&mut self) {
        if self.entries.is_empty() {
            return;
        }

        self.entries.remove(self.active);
        self.active = self.active.saturating_sub(1);
    }

    /// Closes all buffers other than the currently selected one
    pub fn close_other(&mut self) {
        if self.entries.is_empty() {
            return;
        }

        let entry = self.entries.remove(self.active);
        self.entries.clear();

        self.active = 0;

        self.entries.push(entry);
    }

    /// Moves the primary selection the given distance
    ///
    /// Wraps if going `< 0` or `> list length`
    pub fn move_primary(&mut self, dist: isize) {
        if self.entries.is_empty() {
            return;
        }

        let optional = self.active as isize + dist;
        self.active = optional.rem_euclid(self.entries.len() as isize) as usize;
    }

    /// Remove the id from the list
    pub fn remove(&mut self, id: ID) {
        self.entries.retain(|(x, _)| *x != id);
    }

    /// Updates the List to match the Storage's stored IDs
    pub fn update(&mut self, storage: &Storage) {
        self.entries.retain(|(x, _)| storage.has(x));

        self.active = self.active.min(self.entries.len().saturating_sub(1));
    }
}
