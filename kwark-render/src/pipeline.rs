use std::rc::Rc;

use kui::{Node, Painter, Point, Rect};
use kwark_text::Cursor;
use kwark_tree_sitter::tree::SyntaxTree;
use ropey::Rope;

use crate::{
    Border, Caret, Carets, ElementGrid, Gutters, Highlight, Highlighters, LineCtx, Metrics,
    Renderers, Wrapper, line_carets, plain_col,
};

/// The configuration for how to render a buffer to the screen
pub struct Pipeline<S> {
    pub tab: Rc<str>,
    pub metrics: Metrics,

    pub highlighters: Highlighters<S>,
    pub renderers: Renderers<S>,
    pub gutters: Gutters<S>,
    pub carets: Carets<S>,
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
            carets: Carets::default(),
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
    /// The cell of the primary caret, when it was on screen
    pub caret: Option<Rect>,
}

pub struct RenderArgs<'a, 'p, S> {
    pub rope: &'a Rope,
    pub state: &'a mut S,
    pub scroll_top: usize,
    pub cursors: &'a [Cursor],
    pub primary: usize,
    pub tree: Option<&'a SyntaxTree>,
    pub area: Rect,
    /// Whether the window has focus
    pub active: bool,
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
    active: bool,
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
            active,
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
            active,
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

        let mut primary_caret = None;

        for row in &rows {
            if row.gutter.size.width > 0.0 {
                self.gutters
                    .render(state, &row.ctx, &row.hl, row.gutter, painter);
            }

            self.renderers
                .render(state, &row.ctx, &row.hl, row.text, painter);

            let carets = self.carets_of(state, row);
            if let Some(caret) = carets.iter().find(|c| c.primary) {
                primary_caret = Some(caret.rect);
            }

            if !carets.is_empty() {
                painter.clip(row.text, |p| {
                    self.carets.render(state, &row.ctx, &row.hl, &carets, p);
                });
            }
        }

        Viewport {
            top_line: rows.first().map(|r| r.ctx.line).unwrap_or(scroll_top),
            bottom_line: rows.last().map(|r| r.ctx.line).unwrap_or(scroll_top),
            caret: primary_caret,
        }
    }

    /// Returns the cell of every caret on a row that fits on screen
    fn carets_of(&mut self, state: &mut S, row: &Row) -> Vec<Caret> {
        let (ctx, hl, area) = (&row.ctx, &row.hl, row.text);
        let char_width = ctx.metrics.char_width;

        line_carets(ctx)
            .into_iter()
            .filter_map(|(col, primary)| {
                let at = self.renderers.cell(state, ctx, hl, area, col)?;

                // The next column's left edge, unless it wraps or falls off screen
                let width = self
                    .renderers
                    .cell(state, ctx, hl, area, col + 1)
                    .filter(|next| next.y == at.y && next.x > at.x)
                    .map_or(char_width, |next| next.x - at.x);

                Some(Caret {
                    col,
                    rect: Rect::new(at.x, at.y, width, ctx.metrics.line_height),
                    primary,
                })
            })
            .collect()
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
            active: false,
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
            active: false,
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
        active,
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
            active,
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
    wrapper: Option<Box<dyn Wrapper<S>>>,
}

impl<S, C> Default for EditorPipeline<S, C> {
    fn default() -> Self {
        Self {
            top: ElementGrid::default(),
            bottom: ElementGrid::default(),
            wrapper: None,
        }
    }
}

impl<S: 'static, C> EditorPipeline<S, C> {
    /// Returns the editor's top and bottom rows
    pub fn view(&mut self, state: &mut S, ctx: &C) -> (Vec<Node<S>>, Vec<Node<S>>) {
        (self.top.view(state, ctx), self.bottom.view(state, ctx))
    }

    /// Sets the node that the editor sits inside, replacing any before it
    pub fn set_wrapper(&mut self, wrapper: impl Wrapper<S> + 'static) -> &mut Self {
        self.wrapper = Some(Box::new(wrapper));
        self
    }

    /// Removes the wrapper, so that the editor fills the window
    pub fn clear_wrapper(&mut self) -> &mut Self {
        self.wrapper = None;
        self
    }

    /// Returns the editor's tree inside the wrapper, if there is one
    pub fn wrap(&mut self, state: &mut S, editor: Node<S>) -> Node<S> {
        match &mut self.wrapper {
            Some(wrapper) => wrapper.wrap(state, editor),
            None => editor,
        }
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

    /// Returns the carets on each line for one cursor at a line and column
    fn carets(text: &str, line: usize, col: usize) -> Vec<Vec<Caret>> {
        let mut entry = kwark_text::BufferEntry::new_scratch("t");
        entry.buffer.insert(0, 0, text).unwrap();

        let mut set = kwark_text::CursorSet::new();
        set.set(&entry.buffer, line, col, &kwark_text::CursorOptions::new());

        let mut pipeline = Pipeline::<()>::default();
        let metrics = pipeline.metrics.clone();
        let source = Source {
            rope: entry.buffer.rope(),
            scroll_top: 0,
            cursors: set.cursors(),
            primary: set.primary(),
            tree: None,
            area: area(&pipeline),
            active: true,
        };
        let rows = layout(
            &mut pipeline.highlighters,
            &mut pipeline.renderers,
            &pipeline.gutters,
            &pipeline.tab.clone(),
            &metrics,
            &mut (),
            source,
        );

        rows.iter()
            .map(|row| pipeline.carets_of(&mut (), row))
            .collect()
    }

    #[test]
    fn carets_sit_on_the_cell_of_their_column() {
        let pipeline = Pipeline::<()>::default();
        let m = &pipeline.metrics;

        let lines = carets("ab\tc\nxy", 0, 2);
        assert_eq!(lines[1], []);

        let caret = lines[0][0];
        assert!(caret.primary);
        assert_eq!(caret.col, 2);
        assert_eq!(caret.rect.left(), 2.0 * m.char_width);
        assert!((caret.rect.size.width - 3.0 * m.char_width).abs() < 0.01);
        assert_eq!(caret.rect.size.height, m.line_height);
    }

    #[test]
    fn carets_cover_wide_characters_and_line_ends() {
        let pipeline = Pipeline::<()>::default();
        let m = &pipeline.metrics;

        let wide = carets("名x", 0, 0)[0][0];
        assert!(wide.rect.size.width > m.char_width * 1.5);

        // Past the last character of the file, with no newline to sit on
        let end = carets("ab", 0, 2)[0][0];
        assert_eq!(end.col, 2);
        assert_eq!(end.rect.size.width, m.char_width);

        let empty = carets("ab\n\ncd", 1, 0);
        assert_eq!(empty[1][0].rect.left(), 0.0);
        assert_eq!(empty[1][0].rect.top(), m.line_height);
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
