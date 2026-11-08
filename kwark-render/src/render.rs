use std::ops::Range;
use std::sync::Arc;

use kui::{Color, Painter, Point, Rect, Span, TextLayout, TextStyle};

use crate::{Highlight, LineCtx};

/// Takes in highlights, and allows for custom rendering of text
pub trait Renderer<S> {
    /// Height in logical pixels this line needs, None to leave the line to later renderers
    fn claim(&mut self, state: &mut S, ctx: &LineCtx, hl: &Highlight) -> Option<f32>;

    /// Draws the line, not taking more than `area`
    fn render(&mut self, state: &mut S, ctx: &LineCtx, hl: &Highlight, area: Rect, p: &mut Painter);

    /// Column within the line for a point inside this renderer's claimed row, None for the plain layout
    fn hit_col(
        &mut self,
        _state: &mut S,
        _ctx: &LineCtx,
        _hl: &Highlight,
        _area: Rect,
        _point: Point,
    ) -> Option<usize> {
        None
    }

    /// Top left of a column within this renderer's claimed row
    fn cell(
        &mut self,
        _state: &mut S,
        ctx: &LineCtx,
        hl: &Highlight,
        area: Rect,
        col: usize,
    ) -> Option<Point> {
        plain_cell(ctx, hl, area, col)
    }
}

/// A set of renderers
pub struct Renderers<S>(Vec<Box<dyn Renderer<S>>>);

impl<S> Default for Renderers<S> {
    fn default() -> Self {
        Self(vec![])
    }
}

impl<S> Renderers<S> {
    /// Appends a Renderer to the end
    pub fn push(&mut self, r: impl Renderer<S> + 'static) -> &mut Self {
        self.0.push(Box::new(r));
        self
    }

    /// Height in logical pixels the line expects
    pub fn measure(&mut self, state: &mut S, ctx: &LineCtx, hl: &Highlight) -> f32 {
        self.0
            .iter_mut()
            .find_map(|r| r.claim(state, ctx, hl))
            .unwrap_or(ctx.metrics.line_height)
    }

    fn winner(&mut self, state: &mut S, ctx: &LineCtx, hl: &Highlight) -> Option<usize> {
        self.0
            .iter_mut()
            .position(|r| r.claim(state, ctx, hl).is_some())
    }

    /// Draws the given line
    pub fn render(
        &mut self,
        state: &mut S,
        ctx: &LineCtx,
        hl: &Highlight,
        area: Rect,
        p: &mut Painter,
    ) {
        match self.winner(state, ctx, hl) {
            Some(i) => self.0[i].render(state, ctx, hl, area, p),
            None => render_plain(ctx, hl, &[], &[("\t", &ctx.tab)], area, p),
        }
    }

    /// Column within the line for a point inside the row the chain claimed
    pub fn hit_col(
        &mut self,
        state: &mut S,
        ctx: &LineCtx,
        hl: &Highlight,
        area: Rect,
        point: Point,
    ) -> Option<usize> {
        let i = self.winner(state, ctx, hl)?;
        self.0[i].hit_col(state, ctx, hl, area, point)
    }

    /// Top left of a column within the row the chain claimed
    pub fn cell(
        &mut self,
        state: &mut S,
        ctx: &LineCtx,
        hl: &Highlight,
        area: Rect,
        col: usize,
    ) -> Option<Point> {
        match self.winner(state, ctx, hl) {
            Some(i) => self.0[i].cell(state, ctx, hl, area, col),
            None => plain_cell(ctx, hl, area, col),
        }
    }
}

/// A run of drawn text with a background
struct Background {
    /// Display byte range
    range: Range<usize>,
    color: Color,
}

/// A line laid out as plain text
pub struct PlainLine {
    pub layout: Arc<TextLayout>,
    /// Display byte offset of each char column, with one more entry for the end of the line
    offsets: Vec<usize>,
    backgrounds: Vec<Background>,
    /// Background for the column past the end of the text, from a highlight that reaches it
    trailing: Option<Color>,
}

impl PlainLine {
    /// Lays out a line
    ///
    /// # Args
    /// - `conceal` - Char ranges to leave out
    /// - `replace` - Characters to draw as other text, such as a tab as spaces
    pub fn new(
        ctx: &LineCtx,
        hl: &Highlight,
        conceal: &[Range<usize>],
        replace: &[(&str, &str)],
    ) -> Self {
        let len = ctx.text.len_chars();
        let base = &ctx.metrics.text;

        let mut points: Vec<usize> = hl
            .spans
            .iter()
            .flat_map(|span| [span.range.start, span.range.end])
            .chain(conceal.iter().flat_map(|range| [range.start, range.end]))
            .collect();
        points.push(0);
        points.push(len);
        points.retain(|&p| p <= len);
        points.sort_unstable();
        points.dedup();

        let mut text = String::new();
        let mut styles: Vec<(Range<usize>, TextStyle)> = Vec::new();
        let mut backgrounds = Vec::new();
        let mut offsets = vec![0; len + 1];

        for pair in points.windows(2) {
            let (start, end) = (pair[0], pair[1]);
            let from = text.len();

            let hidden = conceal
                .iter()
                .any(|range| range.start <= start && end <= range.end);

            for (i, c) in ctx.text.slice(start..end).chars().enumerate() {
                offsets[start + i] = text.len();
                if hidden {
                    continue;
                }

                let mut buf = [0; 4];
                let s: &str = match c {
                    '\n' | '\r' => " ",
                    _ => c.encode_utf8(&mut buf),
                };
                let s = replace.iter().find(|r| r.0 == s).map_or(s, |r| r.1);
                text.push_str(s);
            }

            if text.len() > from {
                let style = hl.style_at(start);
                if let Some(bg) = style.bg {
                    backgrounds.push(Background {
                        range: from..text.len(),
                        color: bg,
                    });
                }
                styles.push((from..text.len(), style.text(base)));
            }
        }
        offsets[len] = text.len();

        let line_start = ctx.rope.line_to_char(ctx.line);
        let at_true_eof = line_start + len == ctx.rope.len_chars();
        let reaches_end = hl.spans.iter().any(|span| span.range.start == len);
        let trailing = (at_true_eof && reaches_end)
            .then(|| hl.style_at(len).bg)
            .flatten();

        let spans: Vec<Span> = styles
            .iter()
            .map(|(range, style)| Span::new(&text[range.clone()], style.clone()))
            .collect();
        let spans = if spans.is_empty() {
            vec![Span::new("", base.clone())]
        } else {
            spans
        };

        Self {
            layout: ctx.metrics.fonts.line(&spans),
            offsets,
            backgrounds,
            trailing,
        }
    }

    /// Returns the x offset of a char column, counting past the end in character widths
    pub fn x_of(&self, ctx: &LineCtx, col: usize) -> f32 {
        match self.offsets.get(col) {
            Some(&offset) => self.layout.x_for_index(offset),
            None => {
                let extra = (col + 1 - self.offsets.len()) as f32;
                self.layout.width() + extra * ctx.metrics.char_width
            }
        }
    }

    /// Returns the char column under an x offset, counting past the end in character widths
    pub fn col_at(&self, ctx: &LineCtx, x: f32) -> usize {
        let width = self.layout.width();
        let len = self.offsets.len() - 1;

        if x >= width {
            return len + ((x - width) / ctx.metrics.char_width) as usize;
        }

        // Rounds down, so that any point inside a character picks that character
        let index = self.layout.index_for_x(x);
        let col = self.offsets.partition_point(|&o| o < index).min(len);
        if col > 0 && self.layout.x_for_index(self.offsets[col]) > x {
            col - 1
        } else {
            col
        }
    }

    /// Draws the line's backgrounds and text at the top left of `area`
    pub fn paint(&self, ctx: &LineCtx, area: Rect, p: &mut Painter) {
        let height = ctx.metrics.line_height;
        let x = |offset: usize| area.left() + self.layout.x_for_index(offset);

        p.clip(area, |p| {
            for bg in &self.backgrounds {
                let left = x(bg.range.start);
                let rect = Rect::new(left, area.top(), x(bg.range.end) - left, height);
                p.fill(rect, bg.color);
            }

            if let Some(color) = self.trailing {
                let left = area.left() + self.layout.width();
                let rect = Rect::new(left, area.top(), ctx.metrics.char_width, height);
                p.fill(rect, color);
            }

            p.text(&self.layout, Point::new(area.left(), area.top()));
        });
    }
}

/// Top left of a column of a line drawn by `render_plain`, None when it does not fit
pub fn plain_cell(ctx: &LineCtx, hl: &Highlight, area: Rect, col: usize) -> Option<Point> {
    if col > ctx.text.len_chars() {
        return None;
    }

    let line = PlainLine::new(ctx, hl, &[], &[("\t", &ctx.tab)]);
    let x = line.x_of(ctx, col);

    (x + ctx.metrics.char_width <= area.size.width + 0.5)
        .then(|| Point::new(area.left() + x, area.top()))
}

/// Column of a line drawn by `render_plain` under a point
pub fn plain_col(ctx: &LineCtx, hl: &Highlight, area: Rect, point: Point) -> usize {
    let line = PlainLine::new(ctx, hl, &[], &[("\t", &ctx.tab)]);
    line.col_at(ctx, (point.x - area.left()).max(0.0))
}

/// Draws a line as plain text
///
/// # Args
/// - `conceal` - Char ranges to leave out
/// - `replace` - Characters to draw as other text, such as a tab as spaces
pub fn render_plain(
    ctx: &LineCtx,
    hl: &Highlight,
    conceal: &[Range<usize>],
    replace: &[(&str, &str)],
    area: Rect,
    p: &mut Painter,
) {
    PlainLine::new(ctx, hl, conceal, replace).paint(ctx, area, p);
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use kui::{Fonts, TextStyle};
    use kwark_theme::Style;
    use ropey::Rope;

    use super::*;
    use crate::{Metrics, StyleSpan};

    fn metrics() -> Metrics {
        Metrics::new(Fonts::new(), TextStyle::default())
    }

    fn ctx<'a>(rope: &'a Rope, metrics: &'a Metrics) -> LineCtx<'a> {
        LineCtx {
            rope,
            line: 0,
            tab: Rc::from("   "),
            text: rope.line(0),
            width: 800.0,
            cursors: &[],
            primary: 0,
            tree: None,
            metrics,
        }
    }

    /// Returns a rect a number of characters wide
    fn cols(m: &Metrics, x: f32, width: f32) -> Rect {
        Rect::new(x * m.char_width, 0.0, width * m.char_width, m.line_height)
    }

    /// Returns the char column under the middle of a character cell
    fn col_at(ctx: &LineCtx, area: Rect, cell: f32) -> usize {
        let x = area.left() + (cell + 0.5) * ctx.metrics.char_width;
        plain_col(ctx, &Highlight::default(), area, Point::new(x, 0.0))
    }

    struct Fixed(Option<f32>);

    impl<S> Renderer<S> for Fixed {
        fn claim(&mut self, _state: &mut S, _ctx: &LineCtx, _hl: &Highlight) -> Option<f32> {
            self.0
        }

        fn render(
            &mut self,
            _state: &mut S,
            _ctx: &LineCtx,
            _hl: &Highlight,
            _area: Rect,
            _p: &mut Painter,
        ) {
        }
    }

    #[test]
    fn no_claim_measures_as_one_line() {
        let m = metrics();
        let rope = Rope::from_str("hello");
        let mut chain = Renderers::default();

        chain.push(Fixed(None));

        assert_eq!(
            chain.measure(&mut (), &ctx(&rope, &m), &Highlight::default()),
            m.line_height
        );
    }

    #[test]
    fn first_claim_wins_the_measure() {
        let m = metrics();
        let rope = Rope::from_str("hello");
        let mut chain = Renderers::default();

        chain.push(Fixed(None));
        chain.push(Fixed(Some(40.0)));
        chain.push(Fixed(Some(90.0)));

        assert_eq!(
            chain.measure(&mut (), &ctx(&rope, &m), &Highlight::default()),
            40.0
        );
    }

    #[test]
    fn conceal_ranges_hide_text() {
        let m = metrics();
        let rope = Rope::from_str("**bold**");
        let line_ctx = ctx(&rope, &m);
        let line = PlainLine::new(&line_ctx, &Highlight::default(), &[0..2, 6..8], &[]);

        assert_eq!(line.layout.len(), 4);
        assert_eq!(line.x_of(&line_ctx, 2), 0.0);
    }

    #[test]
    fn spans_with_a_background_fill_behind_their_text() {
        let m = metrics();
        let green = Color::rgb(0x00FF00);
        let rope = Rope::from_str("abc");
        let line_ctx = ctx(&rope, &m);
        let hl = Highlight::plain(vec![
            StyleSpan {
                range: 1..2,
                style: Style::new().bg(green),
            },
            StyleSpan {
                range: 3..4,
                style: Style::new().bg(green),
            },
        ]);

        let line = PlainLine::new(&line_ctx, &hl, &[], &[]);

        assert_eq!(line.backgrounds.len(), 1);
        assert_eq!(line.backgrounds[0].range, 1..2);
        assert_eq!(line.trailing, Some(green));
    }

    #[test]
    fn plain_cell_counts_wide_characters() {
        let m = metrics();
        let rope = Rope::from_str("let 名 = x;");
        let line_ctx = ctx(&rope, &m);
        let area = cols(&m, 4.0, 20.0);
        let hl = Highlight::default();

        let wide = plain_cell(&line_ctx, &hl, area, 5).unwrap().x;
        let narrow = plain_cell(&line_ctx, &hl, area, 4).unwrap().x;

        assert!(wide - narrow > m.char_width * 1.5);
    }

    #[test]
    fn tabs_take_the_width_of_the_tab_string() {
        let m = metrics();
        let rope = Rope::from_str("\tab");
        let line_ctx = ctx(&rope, &m);
        let area = cols(&m, 0.0, 6.0);
        let hl = Highlight::default();

        assert_eq!(
            plain_cell(&line_ctx, &hl, area, 0),
            Some(Point::new(0.0, 0.0))
        );
        assert_eq!(
            plain_cell(&line_ctx, &hl, area, 1),
            Some(Point::new(3.0 * m.char_width, 0.0))
        );
    }

    #[test]
    fn plain_col_inside_a_tab_is_the_tab() {
        let m = metrics();
        let rope = Rope::from_str("\tab");
        let line_ctx = ctx(&rope, &m);
        let area = cols(&m, 0.0, 6.0);

        for cell in 0..3 {
            assert_eq!(col_at(&line_ctx, area, cell as f32), 0);
        }
        assert_eq!(col_at(&line_ctx, area, 3.0), 1);
    }

    #[test]
    fn plain_cell_past_the_edge_is_none() {
        let m = metrics();
        let rope = Rope::from_str("hello");
        let line_ctx = ctx(&rope, &m);
        let area = cols(&m, 0.0, 3.0);
        let hl = Highlight::default();

        assert!(plain_cell(&line_ctx, &hl, area, 2).is_some());
        assert_eq!(plain_cell(&line_ctx, &hl, area, 4), None);
    }

    #[test]
    fn plain_cell_end_of_line_is_the_next_cell() {
        let m = metrics();
        let rope = Rope::from_str("abc");
        let line_ctx = ctx(&rope, &m);
        let hl = Highlight::default();

        assert_eq!(
            plain_cell(&line_ctx, &hl, cols(&m, 0.0, 4.0), 3),
            Some(Point::new(3.0 * m.char_width, 0.0))
        );
        assert_eq!(plain_cell(&line_ctx, &hl, cols(&m, 0.0, 3.0), 3), None);
        assert_eq!(plain_cell(&line_ctx, &hl, cols(&m, 0.0, 9.0), 4), None);
    }

    #[test]
    fn plain_col_inverts_plain_cell() {
        let m = metrics();
        let rope = Rope::from_str("let\t名 = x;");
        let line_ctx = ctx(&rope, &m);
        let area = cols(&m, 4.0, 20.0);
        let hl = Highlight::default();

        for col in 0..line_ctx.text.len_chars() {
            let cell = plain_cell(&line_ctx, &hl, area, col).unwrap();
            let inside = Point::new(cell.x + 1.0, cell.y);
            assert_eq!(plain_col(&line_ctx, &hl, area, inside), col);
        }
    }

    #[test]
    fn plain_col_past_the_text_keeps_counting() {
        let m = metrics();
        let rope = Rope::from_str("ab");
        let line_ctx = ctx(&rope, &m);

        assert_eq!(col_at(&line_ctx, cols(&m, 0.0, 10.0), 5.0), 5);
    }
}
