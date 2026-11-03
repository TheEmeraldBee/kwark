mod layout;
pub use layout::*;

use std::ops::{Deref, DerefMut};

use kwark_buffer as buffer;
use kwark_core::{Pipeline, State};
use kwark_render::{Boundaries, Viewport, split_vertically};

use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::widgets::{Clear, Widget};

pub type WindowPipeline = kwark_render::WindowPipeline<State, WindowCtx>;

/// An event when the focused window changes
#[derive(Debug, Clone, Copy)]
pub struct FocusChanged {
    pub from: LeafId,
    pub to: LeafId,
}

/// state carried by a [`Window`]'s [`buffer::List`]
#[derive(Default, Clone, Copy)]
pub struct BufView {
    scroll: usize,
    viewport: Option<Viewport>,
}

/// A buffer's info that's open in the window
pub struct Tab {
    pub name: String,
    pub primary: bool,
}

/// Context passed into each window
pub struct WindowCtx {
    pub active: bool,
    pub boundaries: Boundaries,
    pub tabs: Vec<Tab>,
}

/// A single pane in the editor
#[derive(Default)]
pub struct Window {
    bufs: buffer::List<BufView>,

    rect: Option<Rect>,
    content_rect: Option<Rect>,
}

impl Deref for Window {
    type Target = buffer::List<BufView>;

    fn deref(&self) -> &Self::Target {
        &self.bufs
    }
}

impl DerefMut for Window {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bufs
    }
}

impl Window {
    fn new() -> Self {
        Self {
            bufs: buffer::List::new(),
            rect: None,
            content_rect: None,
        }
    }

    /// The rect the window was drawn to on its last render
    pub fn rect(&self) -> Option<Rect> {
        self.rect
    }

    /// The rect for the window's content
    pub fn content_rect(&self) -> Option<Rect> {
        self.content_rect
    }

    /// Scrolls the selected buffer by `delta` lines, clamped to `0..total_lines`
    pub fn scroll_by(&mut self, delta: isize, total_lines: usize) {
        let Some(view) = self.bufs.state_mut() else {
            return;
        };

        let max = total_lines.saturating_sub(1);
        view.scroll = (view.scroll as isize + delta).clamp(0, max as isize) as usize;
    }

    /// The last rendered viewport for the selected buffer, if it has been drawn
    pub fn viewport(&self) -> Option<Viewport> {
        self.bufs.state().and_then(|v| v.viewport)
    }

    /// Screen cell for a line and column of the selected buffer, None when it is off screen
    pub fn cell_of(
        &self,
        storage: &buffer::Storage,
        pipeline: &mut Pipeline,
        state: &mut State,
        pos: (usize, usize),
    ) -> Option<Position> {
        let rect = self.content_rect?;
        let buf = storage.get(&self.bufs.selected()?)?;
        let scroll = self.bufs.state().map(|v| v.scroll).unwrap_or(0);

        buf.cell_of(pipeline, state, rect, scroll, pos)
    }
}

/// Screen cell for a line and column in a window's selected buffer, None when it is off screen
pub fn cell_of(state: &mut State, leaf: LeafId, pos: (usize, usize)) -> Option<Position> {
    let mut taken = state.scope::<(buffer::Storage, Pipeline, WindowLayout)>();
    let ((storage, pipeline, windows), state) = taken.split();

    windows.cell_of(leaf, storage, pipeline, state, pos)
}
