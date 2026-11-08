use std::rc::Rc;

use kui::{Node, Painter, Point, Rect};
use kwark_text::Cursor;
use kwark_tree_sitter::tree::SyntaxTree;
use ropey::Rope;

use crate::{
    Border, ElementGrid, Gutters, Highlight, Highlighters, LineCtx, Metrics, Renderers, plain_col,
};

/// The configuration for how to render a buffer to the screen
pub struct Pipeline<S> {
    pub tab: Rc<str>,
    pub metrics: Metrics,

    pub highlighters: Highlighters<S>,
    pub renderers: Renderers<S>,
    pub gutters: Gutters<S>,
}

impl<S> Default for Pipeline<S> {
    fn default() -> Self {
        Self::new(Metrics::default())
    }
}

impl<S> Pipeline<S> {
    /// Returns an empty pipeline that lays out text with the given metrics
    pub fn new(metrics: Metrics) -> Self {
        Self {
            tab: Rc::from("   "),
            metrics,
            highlighters: Highlighters::default(),
            renderers: Renderers::default(),
            gutters: Gutters::default(),
        }
    }

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

pub struct RenderArgs<'a, 'p, S> {
    pub rope: &'a Rope,
    pub state: &'a mut S,
    pub scroll_top: usize,
    pub cursors: &'a [Cursor],
    pub primary: usize,
    pub tree: Option<&'a SyntaxTree>,
    pub area: Rect,
    pub painter: &'a mut Painter<'p>,
}

pub struct HitArgs<'a, S> {
    pub rope: &'a Rope,
    pub state: &'a mut S,
    pub scroll_top: usize,
    pub cursors: &'a [Cursor],
    pub primary: usize,
    pub tree: Option<&'a SyntaxTree>,
    pub area: Rect,
    pub point: Point,
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
            painter,
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
        let metrics = self.metrics.clone();
        let rows = layout(
            &mut self.highlighters,
            &mut self.renderers,
            &self.gutters,
            &self.tab,
            &metrics,
            state,
            source,
        );

        for row in &rows {
            if row.gutter.size.width > 0.0 {
                self.gutters
                    .render(state, &row.ctx, &row.hl, row.gutter, painter);
            }

            self.renderers
                .render(state, &row.ctx, &row.hl, row.text, painter);
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
        if !area.contains(point) || point.x < area.left() + self.gutters.measure(&self.metrics) {
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
        let metrics = self.metrics.clone();
        let rows = layout(
            &mut self.highlighters,
            &mut self.renderers,
            &self.gutters,
            &self.tab,
            &metrics,
            state,
            source,
        );
        let row = rows.iter().find(|r| point.y < r.text.bottom())?;

        let col = self
            .renderers
            .hit_col(state, &row.ctx, &row.hl, row.text, point)
            .unwrap_or_else(|| plain_col(&row.ctx, &row.hl, row.text, point));

        Some((row.ctx.line, col))
    }

    /// Top left of a buffer line and column, None when it is off screen
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
    ) -> Option<Point> {
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
        let metrics = self.metrics.clone();
        let rows = layout(
            &mut self.highlighters,
            &mut self.renderers,
            &self.gutters,
            &self.tab,
            &metrics,
            state,
            source,
        );
        let row = rows.iter().find(|r| r.ctx.line == line)?;

        let pos = self
            .renderers
            .cell(state, &row.ctx, &row.hl, row.text, col)?;

        row.text.contains(pos).then_some(pos)
    }
}

/// Lines on screen from scroll_top and the rects they take
fn layout<'a, S>(
    highlighters: &mut Highlighters<S>,
    renderers: &mut Renderers<S>,
    gutters: &Gutters<S>,
    tab: &Rc<str>,
    metrics: &'a Metrics,
    state: &mut S,
    source: Source<'a>,
) -> Vec<Row<'a>> {
    let Source {
        rope,
        scroll_top,
        cursors,
        primary,
        tree,
        area,
    } = source;

    let gutter_width = gutters.measure(metrics).min(area.size.width);
    let text_width = area.size.width - gutter_width;

    let mut rows = Vec::new();
    let mut used = 0.0;
    let mut line = scroll_top;

    while used < area.size.height && line < rope.len_lines() {
        let ctx = LineCtx {
            rope,
            line,
            tab: tab.clone(),
            text: rope.line(line),
            width: text_width,
            cursors,
            primary,
            tree,
            metrics,
        };

        let hl = highlighters.resolve(state, &ctx);
        let height = renderers
            .measure(state, &ctx, &hl)
            .min(area.size.height - used);
        let y = area.top() + used;

        rows.push(Row {
            ctx,
            hl,
            gutter: Rect::new(area.left(), y, gutter_width, height),
            text: Rect::new(area.left() + gutter_width, y, text_width, height),
        });

        used += height;
        line += 1;
    }

    rows
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

impl<S: 'static, C> EditorPipeline<S, C> {
    /// Returns the editor's top and bottom rows
    pub fn view(&mut self, state: &mut S, ctx: &C) -> (Vec<Node<S>>, Vec<Node<S>>) {
        (self.top.view(state, ctx), self.bottom.view(state, ctx))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Gutter, Renderer};

    /// Claims three lines of height for the first line only
    struct Tall;

    impl Renderer<()> for Tall {
        fn claim(&mut self, _state: &mut (), ctx: &LineCtx, _hl: &Highlight) -> Option<f32> {
            (ctx.line == 0).then_some(ctx.metrics.line_height * 3.0)
        }

        fn render(
            &mut self,
            _state: &mut (),
            _ctx: &LineCtx,
            _hl: &Highlight,
            _area: Rect,
            _p: &mut Painter,
        ) {
        }
    }

    /// A gutter some characters wide
    struct Blank(f32);

    impl Gutter<()> for Blank {
        fn width(&self, metrics: &Metrics) -> f32 {
            self.0 * metrics.char_width
        }

        fn render(
            &mut self,
            _state: &mut (),
            _ctx: &LineCtx,
            _hl: &Highlight,
            _area: Rect,
            _p: &mut Painter,
        ) {
        }
    }

    /// Returns an area ten characters wide and four lines tall
    fn area(p: &Pipeline<()>) -> Rect {
        let m = &p.metrics;
        Rect::new(0.0, 0.0, 10.0 * m.char_width, 4.0 * m.line_height)
    }

    fn cell_of(
        pipeline: &mut Pipeline<()>,
        rope: &Rope,
        scroll_top: usize,
        line: usize,
        col: usize,
    ) -> Option<Point> {
        let area = area(pipeline);
        pipeline.cell_of(CellArgs {
            rope,
            state: &mut (),
            scroll_top,
            cursors: &[],
            primary: 0,
            tree: None,
            area,
            line,
            col,
        })
    }

    fn hit_test(pipeline: &mut Pipeline<()>, rope: &Rope, point: Point) -> Option<(usize, usize)> {
        let area = area(pipeline);
        pipeline.hit_test(HitArgs {
            rope,
            state: &mut (),
            scroll_top: 0,
            cursors: &[],
            primary: 0,
            tree: None,
            area,
            point,
        })
    }

    #[test]
    fn lines_off_screen_have_no_cell() {
        let rope = Rope::from_str("a\nb\nc\nd\ne\nf");
        let mut pipeline = Pipeline::default();
        let line_height = pipeline.metrics.line_height;

        assert_eq!(cell_of(&mut pipeline, &rope, 1, 0, 0), None);
        assert_eq!(cell_of(&mut pipeline, &rope, 1, 5, 0), None);
        assert_eq!(
            cell_of(&mut pipeline, &rope, 1, 2, 0),
            Some(Point::new(0.0, line_height))
        );
    }

    #[test]
    fn tall_rows_and_gutters_shift_cells() {
        let rope = Rope::from_str("a\nb\nc");
        let mut pipeline = Pipeline::default();
        let m = pipeline.metrics.clone();

        pipeline.renderers.push(Tall);
        pipeline.gutters.push(Blank(2.0));

        assert_eq!(
            cell_of(&mut pipeline, &rope, 0, 1, 0),
            Some(Point::new(2.0 * m.char_width, 3.0 * m.line_height))
        );
        assert_eq!(cell_of(&mut pipeline, &rope, 0, 2, 0), None);
    }

    #[test]
    fn hit_test_inverts_cell_of() {
        let rope = Rope::from_str("名x\nab");
        let mut pipeline = Pipeline::default();

        pipeline.gutters.push(Blank(1.0));

        for (line, col) in [(0, 0), (0, 1), (1, 1)] {
            let cell = cell_of(&mut pipeline, &rope, 0, line, col).unwrap();
            let inside = Point::new(cell.x + 1.0, cell.y + 1.0);
            assert_eq!(hit_test(&mut pipeline, &rope, inside), Some((line, col)));
        }
    }
}
