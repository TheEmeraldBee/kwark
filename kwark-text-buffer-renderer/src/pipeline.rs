use kwark_text_buffer::Cursor;
use kwark_tree_sitter::tree::SyntaxTree;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::{Clear, Widget};
use ropey::Rope;

use crate::{Border, ElementGrid, Gutters, Highlighters, LineCtx, Renderers};

/// A full set of storage for Renderers and Highlighters
pub struct Pipeline<S> {
    pub highlighters: Highlighters<S>,
    pub renderers: Renderers<S>,
    pub gutters: Gutters<S>,
}

impl<S> Default for Pipeline<S> {
    fn default() -> Self {
        Self {
            highlighters: Highlighters::default(),
            renderers: Renderers::default(),
            gutters: Gutters::default(),
        }
    }
}

/// The range of lines a render actually drew, read back after the fact
#[derive(Clone, Copy, Debug, Default)]
pub struct Viewport {
    pub top_line: usize,
    pub bottom_line: usize,
}

pub struct RenderArgs<'a, S> {
    pub rope: &'a Rope,
    pub state: &'a mut S,
    pub scroll_top: usize,
    pub cursors: &'a [Cursor],
    pub primary: usize,
    pub tree: Option<&'a SyntaxTree>,
    pub area: Rect,
    pub buf: &'a mut Buffer,
}

pub struct HitArgs<'a, S> {
    pub rope: &'a Rope,
    pub state: &'a mut S,
    pub scroll_top: usize,
    pub cursors: &'a [Cursor],
    pub primary: usize,
    pub tree: Option<&'a SyntaxTree>,
    pub area: Rect,
    pub point: Position,
}

impl<S> Pipeline<S> {
    /// Renders buffer lines from scroll_top downward, stopping once area is full
    ///
    /// Returns the range of lines that ended up on screen
    pub fn render(
        &mut self,
        RenderArgs {
            rope,
            state,
            scroll_top,
            cursors,
            primary,
            tree,
            area,
            buf,
        }: RenderArgs<S>,
    ) -> Viewport {
        let gutter_width = self.gutters.measure();
        let text_area = Rect {
            x: area.x + gutter_width,
            y: area.y,
            width: area.width.saturating_sub(gutter_width),
            height: area.height,
        };

        let mut resolved = Vec::new();
        let mut used: u16 = 0;
        let mut line = scroll_top;

        while used < area.height && line < rope.len_lines() {
            let ctx = LineCtx {
                rope,
                line,
                text: rope.line(line),
                width: text_area.width,
                cursors,
                primary,
                tree,
            };

            let hl = self.highlighters.resolve(state, &ctx);
            let height = self
                .renderers
                .measure(state, &ctx, &hl)
                .min(area.height - used);

            used += height;
            resolved.push((ctx, hl, height));
            line += 1;
        }

        let mut y = area.y;

        for (ctx, hl, height) in &resolved {
            let row = Rect {
                x: text_area.x,
                y,
                width: text_area.width,
                height: *height,
            };

            if gutter_width > 0 {
                let gutter_row = Rect {
                    x: area.x,
                    y,
                    width: gutter_width,
                    height: *height,
                };
                self.gutters
                    .render(state, ctx, hl, gutter_row, buf, *height);
            }

            self.renderers.render(state, ctx, hl, row, buf);
            y += height;
        }

        Viewport {
            top_line: resolved.first().map(|(c, ..)| c.line).unwrap_or(scroll_top),
            bottom_line: resolved.last().map(|(c, ..)| c.line).unwrap_or(scroll_top),
        }
    }

    /// Finds the line and column under `point`, replaying the same measuring pass as `render`
    pub fn hit_test(
        &mut self,
        HitArgs {
            rope,
            state,
            scroll_top,
            cursors,
            primary,
            tree,
            area,
            point,
        }: HitArgs<S>,
    ) -> Option<(usize, usize)> {
        if !area.contains(point) {
            return None;
        }

        let gutter_width = self.gutters.measure();
        let text_area = Rect {
            x: area.x + gutter_width,
            y: area.y,
            width: area.width.saturating_sub(gutter_width),
            height: area.height,
        };

        if point.x < text_area.x {
            return None;
        }

        let mut used: u16 = 0;
        let mut y = area.y;
        let mut line = scroll_top;

        while used < area.height && line < rope.len_lines() {
            let ctx = LineCtx {
                rope,
                line,
                text: rope.line(line),
                width: text_area.width,
                cursors,
                primary,
                tree,
            };

            let hl = self.highlighters.resolve(state, &ctx);
            let height = self
                .renderers
                .measure(state, &ctx, &hl)
                .min(area.height - used);
            let row = Rect {
                x: text_area.x,
                y,
                width: text_area.width,
                height,
            };

            if point.y < row.y + row.height {
                let col = self
                    .renderers
                    .hit_col(state, &ctx, &hl, row, point)
                    .unwrap_or_else(|| point.x.saturating_sub(row.x) as usize);

                return Some((line, col));
            }

            used += height;
            y += height;
            line += 1;
        }

        None
    }
}

/// Splits `area` into top, middle, and bottom bands of the given heights
pub fn split_vertically(area: Rect, top: u16, bottom: u16) -> (Rect, Rect, Rect) {
    let middle = area.height.saturating_sub(top + bottom);

    let top_rect = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: top,
    };
    let middle_rect = Rect {
        x: area.x,
        y: area.y + top,
        width: area.width,
        height: middle,
    };
    let bottom_rect = Rect {
        x: area.x,
        y: area.y + top + middle,
        width: area.width,
        height: bottom,
    };

    (top_rect, middle_rect, bottom_rect)
}

/// Arguments for rendering a single window's
pub struct WindowRenderArgs<'a, S, C> {
    pub rope: &'a Rope,
    pub state: &'a mut S,
    pub scroll_top: usize,
    pub cursors: &'a [Cursor],
    pub primary: usize,
    pub tree: Option<&'a SyntaxTree>,
    pub area: Rect,
    pub buf: &'a mut Buffer,
    pub ctx: &'a C,
}

/// A pipeline to run when rendering a window
pub struct WindowPipeline<S, C> {
    pub border: Option<Box<dyn Border<S, C>>>,
    pub top: ElementGrid<S, C>,
    pub bottom: ElementGrid<S, C>,
}

impl<S, C> Default for WindowPipeline<S, C> {
    fn default() -> Self {
        Self {
            border: None,
            top: ElementGrid::default(),
            bottom: ElementGrid::default(),
        }
    }
}

impl<S, C> WindowPipeline<S, C> {
    /// Sets the border renderer for the pipeline
    pub fn set_border(&mut self, border: impl Border<S, C> + 'static) -> &mut Self {
        self.border = Some(Box::new(border));
        self
    }

    /// Removes the border renderer from the pipeline
    pub fn remove_border(&mut self) -> &mut Self {
        self.border = None;
        self
    }

    /// Draws the window's elements
    pub fn render(
        &mut self,
        content: &mut Pipeline<S>,
        WindowRenderArgs {
            rope,
            state,
            scroll_top,
            cursors,
            primary,
            tree,
            area,
            buf,
            ctx,
        }: WindowRenderArgs<S, C>,
    ) -> Viewport {
        let inner = match &mut self.border {
            Some(border) => {
                let inner = border.render(state, ctx, area, buf);
                // The border may have styled the whole `area`; clear its own
                // styling back out of `inner` before anything draws there
                Clear.render(inner, buf);
                inner
            }
            None => area,
        };

        let top_h = self.top.height(state, ctx, inner.height);
        let bottom_h = self
            .bottom
            .height(state, ctx, inner.height.saturating_sub(top_h));
        let (top_rect, content_rect, bottom_rect) = split_vertically(inner, top_h, bottom_h);

        self.top.render(state, ctx, top_rect, buf);
        self.bottom.render(state, ctx, bottom_rect, buf);

        content.render(RenderArgs {
            rope,
            state,
            scroll_top,
            cursors,
            primary,
            tree,
            area: content_rect,
            buf,
        })
    }
}

/// The editor's render pipeline
pub struct EditorPipeline<S, C> {
    pub top: ElementGrid<S, C>,
    pub bottom: ElementGrid<S, C>,
}

impl<S, C> Default for EditorPipeline<S, C> {
    fn default() -> Self {
        Self {
            top: ElementGrid::default(),
            bottom: ElementGrid::default(),
        }
    }
}

impl<S, C> EditorPipeline<S, C> {
    /// Draws the editor's elements, returning what space is available for windows
    pub fn render(&mut self, state: &mut S, ctx: &C, area: Rect, buf: &mut Buffer) -> Rect {
        let top_h = self.top.height(state, ctx, area.height);
        let bottom_h = self
            .bottom
            .height(state, ctx, area.height.saturating_sub(top_h));
        let (top_rect, grid_rect, bottom_rect) = split_vertically(area, top_h, bottom_h);

        self.top.render(state, ctx, top_rect, buf);
        self.bottom.render(state, ctx, bottom_rect, buf);

        grid_rect
    }
}
