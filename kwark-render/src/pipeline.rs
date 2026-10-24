use std::rc::Rc;

use kwark_text::Cursor;
use kwark_tree_sitter::tree::SyntaxTree;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::{Clear, Widget};
use ropey::Rope;

use crate::{Border, ElementGrid, Gutters, Highlight, Highlighters, LineCtx, Renderers, plain_col};

/// The configuration for how to render a buffer to the screen
pub struct Pipeline<S> {
    pub tab: Rc<str>,

    pub highlighters: Highlighters<S>,
    pub renderers: Renderers<S>,
    pub gutters: Gutters<S>,
}

impl<S> Default for Pipeline<S> {
    fn default() -> Self {
        Self {
            tab: Rc::from("   "),
            highlighters: Highlighters::default(),
            renderers: Renderers::default(),
            gutters: Gutters::default(),
        }
    }
}

impl<S> Pipeline<S> {
    /// Sets the tab-characters for the renderer
    pub fn set_tab(&mut self, tab: &str) {
        self.tab = Rc::from(tab);
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

pub struct CellArgs<'a, S> {
    pub rope: &'a Rope,
    pub state: &'a mut S,
    pub scroll_top: usize,
    pub cursors: &'a [Cursor],
    pub primary: usize,
    pub tree: Option<&'a SyntaxTree>,
    pub area: Rect,
    pub line: usize,
    pub col: usize,
}

/// The buffer and area a pipeline lays out
#[derive(Clone, Copy)]
struct Source<'a> {
    rope: &'a Rope,
    scroll_top: usize,
    cursors: &'a [Cursor],
    primary: usize,
    tree: Option<&'a SyntaxTree>,
    area: Rect,
}

/// A buffer line and the rects it takes on screen
struct Row<'a> {
    ctx: LineCtx<'a>,
    hl: Highlight,
    gutter: Rect,
    text: Rect,
}

impl<S> Pipeline<S> {
    /// Renders buffer lines from scroll_top and returns the range of lines on screen
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
        let source = Source {
            rope,
            scroll_top,
            cursors,
            primary,
            tree,
            area,
        };
        let rows = self.layout(state, source);

        for row in &rows {
            if row.gutter.width > 0 {
                self.gutters
                    .render(state, &row.ctx, &row.hl, row.gutter, buf, row.gutter.height);
            }

            self.renderers
                .render(state, &row.ctx, &row.hl, row.text, buf);
        }

        Viewport {
            top_line: rows.first().map(|r| r.ctx.line).unwrap_or(scroll_top),
            bottom_line: rows.last().map(|r| r.ctx.line).unwrap_or(scroll_top),
        }
    }

    /// Line and column under `point`, None when it is off the text
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
        if !area.contains(point) || point.x < area.x + self.gutters.measure() {
            return None;
        }

        let source = Source {
            rope,
            scroll_top,
            cursors,
            primary,
            tree,
            area,
        };
        let rows = self.layout(state, source);
        let row = rows.iter().find(|r| point.y < r.text.y + r.text.height)?;

        let col = self
            .renderers
            .hit_col(state, &row.ctx, &row.hl, row.text, point)
            .unwrap_or_else(|| plain_col(&row.ctx, row.text, point));

        Some((row.ctx.line, col))
    }

    /// Screen cell for a buffer line and column, None when it is off screen
    pub fn cell_of(
        &mut self,
        CellArgs {
            rope,
            state,
            scroll_top,
            cursors,
            primary,
            tree,
            area,
            line,
            col,
        }: CellArgs<S>,
    ) -> Option<Position> {
        if line < scroll_top || line >= rope.len_lines() {
            return None;
        }

        let source = Source {
            rope,
            scroll_top,
            cursors,
            primary,
            tree,
            area,
        };
        let rows = self.layout(state, source);
        let row = rows.iter().find(|r| r.ctx.line == line)?;

        let pos = self
            .renderers
            .cell(state, &row.ctx, &row.hl, row.text, col)?;

        row.text.contains(pos).then_some(pos)
    }

    /// Lines on screen from scroll_top and the rects they take
    fn layout<'a>(&mut self, state: &mut S, source: Source<'a>) -> Vec<Row<'a>> {
        let Source {
            rope,
            scroll_top,
            cursors,
            primary,
            tree,
            area,
        } = source;

        let gutter_width = self.gutters.measure().min(area.width);
        let text_width = area.width - gutter_width;

        let mut rows = Vec::new();
        let mut used: u16 = 0;
        let mut line = scroll_top;

        while used < area.height && line < rope.len_lines() {
            let ctx = LineCtx {
                rope,
                line,
                tab: self.tab.clone(),
                text: rope.line(line),
                width: text_width,
                cursors,
                primary,
                tree,
            };

            let hl = self.highlighters.resolve(state, &ctx);
            let height = self
                .renderers
                .measure(state, &ctx, &hl)
                .min(area.height - used);
            let y = area.y + used;

            rows.push(Row {
                ctx,
                hl,
                gutter: Rect::new(area.x, y, gutter_width, height),
                text: Rect::new(area.x + gutter_width, y, text_width, height),
            });

            used += height;
            line += 1;
        }

        rows
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Gutter, Renderer};

    /// Claims three rows for the first line only
    struct Tall;

    impl Renderer<()> for Tall {
        fn claim(&mut self, _state: &mut (), ctx: &LineCtx, _hl: &Highlight) -> Option<u16> {
            (ctx.line == 0).then_some(3)
        }

        fn render(
            &mut self,
            _state: &mut (),
            _ctx: &LineCtx,
            _hl: &Highlight,
            _area: Rect,
            _buf: &mut Buffer,
        ) {
        }
    }

    struct Blank(u16);

    impl Gutter<()> for Blank {
        fn width(&self) -> u16 {
            self.0
        }

        fn render(
            &mut self,
            _state: &mut (),
            _ctx: &LineCtx,
            _hl: &Highlight,
            _area: Rect,
            _buf: &mut Buffer,
        ) {
        }
    }

    fn cell_of(
        pipeline: &mut Pipeline<()>,
        rope: &Rope,
        scroll_top: usize,
        line: usize,
        col: usize,
    ) -> Option<Position> {
        pipeline.cell_of(CellArgs {
            rope,
            state: &mut (),
            scroll_top,
            cursors: &[],
            primary: 0,
            tree: None,
            area: Rect::new(0, 0, 10, 4),
            line,
            col,
        })
    }

    fn hit_test(
        pipeline: &mut Pipeline<()>,
        rope: &Rope,
        point: Position,
    ) -> Option<(usize, usize)> {
        pipeline.hit_test(HitArgs {
            rope,
            state: &mut (),
            scroll_top: 0,
            cursors: &[],
            primary: 0,
            tree: None,
            area: Rect::new(0, 0, 10, 4),
            point,
        })
    }

    #[test]
    fn lines_off_screen_have_no_cell() {
        let rope = Rope::from_str("a\nb\nc\nd\ne\nf");
        let mut pipeline = Pipeline::default();

        assert_eq!(cell_of(&mut pipeline, &rope, 1, 0, 0), None);
        assert_eq!(cell_of(&mut pipeline, &rope, 1, 5, 0), None);
        assert_eq!(
            cell_of(&mut pipeline, &rope, 1, 2, 0),
            Some(Position::new(0, 1))
        );
    }

    #[test]
    fn tall_rows_and_gutters_shift_cells() {
        let rope = Rope::from_str("a\nb\nc");
        let mut pipeline = Pipeline::default();

        pipeline.renderers.push(Tall);
        pipeline.gutters.push(Blank(2));

        assert_eq!(
            cell_of(&mut pipeline, &rope, 0, 1, 0),
            Some(Position::new(2, 3))
        );
        assert_eq!(cell_of(&mut pipeline, &rope, 0, 2, 0), None);
    }

    #[test]
    fn hit_test_inverts_cell_of() {
        let rope = Rope::from_str("名x\nab");
        let mut pipeline = Pipeline::default();

        pipeline.gutters.push(Blank(1));

        for (line, col) in [(0, 0), (0, 1), (1, 1)] {
            let cell = cell_of(&mut pipeline, &rope, 0, line, col).unwrap();
            assert_eq!(hit_test(&mut pipeline, &rope, cell), Some((line, col)));
        }
    }
}
