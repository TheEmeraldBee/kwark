mod layout;
pub use layout::*;

use std::ops::{Deref, DerefMut};

use kwark_buffer as buffer;
use kwark_render::{Boundaries, Pipeline, Viewport, split_vertically};

use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::widgets::{Clear, Widget};

pub type WindowPipeline<S> = kwark_render::WindowPipeline<S, WindowCtx>;

/// An event when the focused window changes
#[derive(Debug, Clone, Copy)]
pub struct FocusChanged {
    pub from: LeafId,
    pub to: LeafId,
}

/// The scroll position of a buffer in a window
#[derive(Default, Debug, Clone, Copy)]
pub struct Scroll {
    /// The line that the user scrolled to
    pub top: usize,
    /// The line drawn at the top, when it differs from `top`
    pub shown: Option<usize>,
}

impl Scroll {
    /// Returns the line drawn at the top of the window
    pub fn shown(&self) -> usize {
        self.shown.unwrap_or(self.top)
    }
}

/// The view of one buffer in one window
#[derive(Default, Debug, Clone, Copy)]
pub struct BufferView {
    pub scroll: Scroll,
    /// The viewport of the buffer's last render in the window
    pub viewport: Option<Viewport>,
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
    bufs: buffer::List<BufferView>,

    rect: Option<Rect>,
    content_rect: Option<Rect>,
}

impl Deref for Window {
    type Target = buffer::List<BufferView>;

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

    /// Returns the view of the selected buffer in this window
    pub fn view(&self) -> Option<&BufferView> {
        self.bufs.state()
    }

    /// Returns the view of the selected buffer in this window, mutably
    pub fn view_mut(&mut self) -> Option<&mut BufferView> {
        self.bufs.state_mut()
    }

    /// Scrolls the selected buffer by `delta` lines, clamped to `0..total_lines`
    pub fn scroll_by(&mut self, delta: isize, total_lines: usize) {
        let Some(view) = self.bufs.state_mut() else {
            return;
        };

        let max = total_lines.saturating_sub(1);
        view.scroll.top = (view.scroll.top as isize + delta).clamp(0, max as isize) as usize;
    }

    /// Returns the line drawn at the top of the selected buffer
    pub fn scroll_shown(&self) -> usize {
        self.view().map_or(0, |v| v.scroll.shown())
    }

    /// The last rendered viewport for the selected buffer, if it has been drawn
    pub fn viewport(&self) -> Option<Viewport> {
        self.view()?.viewport
    }

    /// Screen cell for a line and column of the selected buffer, None when it is off screen
    pub fn cell_of<S>(
        &self,
        storage: &buffer::Storage,
        pipeline: &mut Pipeline<S>,
        state: &mut S,
        pos: (usize, usize),
    ) -> Option<Position> {
        let rect = self.content_rect?;
        let buf = storage.get(&self.bufs.selected()?)?;

        buf.cell_of(pipeline, state, rect, self.scroll_shown(), pos)
    }

    /// Renders the window's content
    #[allow(clippy::too_many_arguments)]
    pub fn render<S>(
        &mut self,
        storage: &buffer::Storage,
        frame: &mut Frame<'_>,
        rect: Rect,

        pipeline: &mut Pipeline<S>,
        window_pipeline: &mut WindowPipeline<S>,
        state: &mut S,

        is_active: bool,
        boundaries: Boundaries,
    ) {
        self.rect = Some(rect);

        let tabs = self
            .bufs
            .iter()
            .map(|(primary, buf)| Tab {
                name: buf.to_string(),
                primary,
            })
            .collect();
        let ctx = WindowCtx {
            active: is_active,
            boundaries,
            tabs,
        };

        let inner = {
            let screen = frame.buffer_mut();
            match &mut window_pipeline.border {
                Some(border) => {
                    let inner = border.render(state, &ctx, rect, screen);
                    // The border may have styled the whole `rect`; clear its
                    // own styling back out of `inner` before anything draws there
                    Clear.render(inner, screen);
                    inner
                }
                None => rect,
            }
        };

        let top_h = window_pipeline.top.height(state, &ctx, inner.height);
        let bottom_h =
            window_pipeline
                .bottom
                .height(state, &ctx, inner.height.saturating_sub(top_h));
        let (top_rect, text_rect, bottom_rect) = split_vertically(inner, top_h, bottom_h);

        {
            let screen = frame.buffer_mut();
            window_pipeline.top.render(state, &ctx, top_rect, screen);
            window_pipeline
                .bottom
                .render(state, &ctx, bottom_rect, screen);
        }

        self.content_rect = Some(text_rect);

        let Some(id) = self.bufs.selected() else {
            return;
        };
        let Some(buf) = storage.get(&id) else {
            return;
        };

        let viewport = buf.render(pipeline, state, frame, text_rect, self.scroll_shown());

        if let Some(view) = self.bufs.state_mut() {
            view.viewport = Some(viewport);
        }
    }
}
