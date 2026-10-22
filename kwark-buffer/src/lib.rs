use std::{
    collections::HashMap,
    fmt::Display,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use kwark_bus::EventSender;
use kwark_render::{HitArgs, Pipeline, RenderArgs, Viewport};
use kwark_text::{self as text, BoundCursorSet, BufferKind};
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

        /// Ticket of the running background parse
        parse_ticket: Option<u64>,
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

            parse_ticket: None,
        }
    }
}

static NEXT_TICKET: AtomicU64 = AtomicU64::new(0);

/// A full parse of a buffer's text
pub struct ParseJob {
    ticket: u64,
    lang: String,
    rope: ropey::Rope,
    version: u128,
    store: LanguageStore,
}

impl ParseJob {
    /// Parses the snapshot
    pub fn run(mut self) -> ParsedTree {
        let tree = SyntaxTree::parse(&mut self.store, &self.lang, &self.rope);

        ParsedTree {
            ticket: self.ticket,
            version: self.version,
            tree,
            store: self.store,
        }
    }
}

/// The result of a [`ParseJob`]
pub struct ParsedTree {
    ticket: u64,
    version: u128,
    tree: core::result::Result<SyntaxTree, kwark_tree_sitter::error::Error>,
    store: LanguageStore,
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

    /// Returns the version of the buffer's text
    pub fn version(&self) -> u128 {
        match self {
            Self::Text { buf, .. } => buf.buffer.version(),
            Self::Widget(_) => 0,
        }
    }

    /// Returns a full parse job if the buffer needs one
    pub fn parse_job(&mut self, store: &LanguageStore) -> Result<Option<ParseJob>> {
        let Self::Text {
            id,
            buf,
            tree: None,
            parse_ticket: parse_ticket @ None,
            ..
        } = self
        else {
            return Ok(None);
        };

        let path = match id {
            text::BufferKind::File(path) => Some(path.as_path()),
            text::BufferKind::Scratch(_) => None,
        };
        let first_line = buf.buffer.rope().get_line(0).map(|l| l.to_string());

        let Some(lang) = store.detect(path, first_line.as_deref()) else {
            return Ok(None);
        };

        let ticket = NEXT_TICKET.fetch_add(1, Ordering::Relaxed);
        let job = ParseJob {
            ticket,
            lang: lang.to_string(),
            rope: buf.buffer.rope().clone(),
            version: buf.buffer.version(),
            store: store.fork()?,
        };

        *parse_ticket = Some(ticket);

        Ok(Some(job))
    }

    /// Installs a finished parse, caught up to the current text
    pub fn install(&mut self, parsed: ParsedTree, store: &mut LanguageStore) -> Result<()> {
        let ParsedTree {
            ticket,
            version,
            tree: parsed_tree,
            store: worker_store,
        } = parsed;

        store.absorb_queries(worker_store);

        let Self::Text {
            buf,
            tree,
            parsed_version,
            parse_ticket,
            ..
        } = self
        else {
            return Ok(());
        };

        if *parse_ticket != Some(ticket) {
            return Ok(());
        }

        *parse_ticket = None;

        let mut parsed_tree = parsed_tree?;

        let current = buf.buffer.version();
        if version != current {
            let edits = buf.buffer.edits_since(version)?;
            parsed_tree.reparse(store, edits, buf.buffer.rope())?;
        }

        *tree = Some(parsed_tree);
        *parsed_version = current;

        Ok(())
    }

    /// Reparses the tree if the buffer's text has changed since the last parse
    pub fn update(&mut self, store: &mut LanguageStore) -> Result<()> {
        match self {
            Self::Text {
                buf,
                tree,
                parsed_version,
                ..
            } => {
                let Some(tree) = tree else {
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

/// An event when a buffer appears in storage
#[derive(Debug, Clone)]
pub struct BufferOpened {
    pub id: ID,
}

/// An event when a buffer's text changes
#[derive(Debug, Clone)]
pub struct BufferChanged {
    pub id: ID,
    pub version: u128,
}

/// An event when a buffer leaves storage
#[derive(Debug, Clone)]
pub struct BufferClosed {
    pub id: ID,
}

/// A basic storage for a list of Buffers
pub struct Storage {
    next_id: usize,
    inner: HashMap<ID, Buffer>,

    /// The text version last reported for each buffer
    reported: HashMap<ID, u128>,
    bus: EventSender,
}

impl Storage {
    /// Creates an empty storage
    pub fn new(bus: EventSender) -> Self {
        Self {
            next_id: 0,
            inner: HashMap::new(),
            reported: HashMap::new(),
            bus,
        }
    }

    fn put(&mut self, id: ID, buf: Buffer) {
        self.reported.insert(id.clone(), buf.version());

        if self.inner.insert(id.clone(), buf).is_some() {
            self.bus.emit(BufferClosed { id: id.clone() });
        }

        self.bus.emit(BufferOpened { id });
    }

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

        self.put(id.clone(), buf);

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

        self.put(id.clone(), buf);

        id
    }

    /// Removes a buffer from storage, returning it if it existed
    pub fn remove(&mut self, id: &ID) -> Option<Buffer> {
        let buf = self.inner.remove(id)?;

        self.reported.remove(id);
        self.bus.emit(BufferClosed { id: id.clone() });

        Some(buf)
    }

    /// Emits a [`BufferChanged`] for each buffer whose text changed since the last call
    pub fn report_changes(&mut self) {
        for (id, buf) in &self.inner {
            let version = buf.version();

            if self.reported.insert(id.clone(), version) != Some(version) {
                self.bus.emit(BufferChanged {
                    id: id.clone(),
                    version,
                });
            }
        }
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

#[cfg(test)]
mod test {
    use super::*;

    fn store() -> LanguageStore {
        let mut store = LanguageStore::new(PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap();
        store
            .register("rust", Language::new(tree_sitter_rust::LANGUAGE))
            .unwrap()
            .with_shebangs(["rustrun"]);
        store
    }

    fn rust_buffer(text: &str) -> Buffer {
        let mut entry = text::BufferEntry::new_scratch("test");
        entry
            .buffer
            .insert(0, 0, "#!/usr/bin/env rustrun\n")
            .unwrap();
        entry.buffer.insert(1, 0, text).unwrap();
        Buffer::from(entry)
    }

    fn insert(buf: &mut Buffer, line: usize, text: &str) {
        let Buffer::Text { buf, .. } = buf else {
            unreachable!()
        };
        buf.buffer.insert(line, 0, text).unwrap();
    }

    fn tree_text(buf: &Buffer) -> Option<(u128, String)> {
        let Buffer::Text {
            tree: Some(tree),
            parsed_version,
            ..
        } = buf
        else {
            return None;
        };
        Some((*parsed_version, tree.root().root_node().to_sexp()))
    }

    fn events(rx: &mut kwark_bus::EventReceiver) -> Vec<&'static str> {
        let mut out = vec![];

        while let Some((_, event)) = rx.try_recv() {
            if event.is::<BufferOpened>() {
                out.push("opened");
            } else if event.is::<BufferChanged>() {
                out.push("changed");
            } else if event.is::<BufferClosed>() {
                out.push("closed");
            }
        }

        out
    }

    #[test]
    pub fn storage_reports_buffer_lifecycle() {
        let (bus, mut rx) = kwark_bus::channel();
        let mut storage = Storage::new(bus);

        let id = storage.insert(text::BufferEntry::new_scratch("a"));
        storage.report_changes();
        assert_eq!(events(&mut rx), ["opened"]);

        insert(storage.get_mut(&id).unwrap(), 0, "x");
        storage.report_changes();
        storage.report_changes();
        assert_eq!(events(&mut rx), ["changed"]);

        storage.insert(text::BufferEntry::new_scratch("a"));
        assert_eq!(events(&mut rx), ["closed", "opened"]);

        storage.remove(&id);
        assert_eq!(events(&mut rx), ["closed"]);
    }

    #[test]
    pub fn parse_job_only_once() {
        let store = store();
        let mut buf = rust_buffer("fn main() {}");

        assert!(buf.parse_job(&store).unwrap().is_some());
        assert!(buf.parse_job(&store).unwrap().is_none());
    }

    #[test]
    pub fn install_catches_up_with_edits() {
        let mut store = store();
        let mut buf = rust_buffer("fn main() {}\n");

        let job = buf.parse_job(&store).unwrap().unwrap();
        insert(&mut buf, 2, "fn other() {}\n");

        let parsed = std::thread::spawn(move || job.run()).join().unwrap();
        buf.install(parsed, &mut store).unwrap();

        let (version, sexp) = tree_text(&buf).unwrap();
        assert_eq!(version, buf.version());
        assert_eq!(sexp.matches("function_item").count(), 2);
    }

    #[test]
    pub fn install_ignores_a_stale_ticket() {
        let mut store = store();
        let mut old = rust_buffer("fn main() {}");
        let mut new = rust_buffer("fn main() {}");

        let stale = old.parse_job(&store).unwrap().unwrap().run();
        new.parse_job(&store).unwrap().unwrap();
        new.install(stale, &mut store).unwrap();

        assert!(tree_text(&new).is_none());
    }
}
