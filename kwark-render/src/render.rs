use std::ops::Range;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::text::{Line, Span as RSpan};
use ratatui::widgets::Widget;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{Highlight, LineCtx};

/// Takes in highlights, and allows for custom rendering of text
pub trait Renderer<S> {
    /// Row height this line needs
    fn claim(&mut self, state: &mut S, ctx: &LineCtx, hl: &Highlight) -> Option<u16>;

    /// Draw the line to the buffer, not taking more than "area"
    fn render(
        &mut self,
        state: &mut S,
        ctx: &LineCtx,
        hl: &Highlight,
        area: Rect,
        buf: &mut Buffer,
    );

    /// Column within the line for a point inside this renderer's claimed row, None for the plain layout
    fn hit_col(
        &mut self,
        _state: &mut S,
        _ctx: &LineCtx,
        _hl: &Highlight,
        _area: Rect,
        _point: Position,
    ) -> Option<usize> {
        None
    }

    /// Screen cell for a column within this renderer's claimed row
    fn cell(
        &mut self,
        _state: &mut S,
        ctx: &LineCtx,
        _hl: &Highlight,
        area: Rect,
        col: usize,
    ) -> Option<Position> {
        plain_cell(ctx, area, col)
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

    /// Row height the line expects
    pub fn measure(&mut self, state: &mut S, ctx: &LineCtx, hl: &Highlight) -> u16 {
        self.0
            .iter_mut()
            .find_map(|r| r.claim(state, ctx, hl))
            .unwrap_or(1)
    }

    /// Renders the given line to the given buffer
    pub fn render(
        &mut self,
        state: &mut S,
        ctx: &LineCtx,
        hl: &Highlight,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let winner = self
            .0
            .iter_mut()
            .position(|r| r.claim(state, ctx, hl).is_some());

        match winner {
            Some(i) => self.0[i].render(state, ctx, hl, area, buf),
            None => render_plain(ctx, hl, &[], &[("\t", &ctx.tab)], area, buf),
        }
    }

    /// Column within the line for a point inside the row the chain claimed
    pub fn hit_col(
        &mut self,
        state: &mut S,
        ctx: &LineCtx,
        hl: &Highlight,
        area: Rect,
        point: Position,
    ) -> Option<usize> {
        let winner = self
            .0
            .iter_mut()
            .position(|r| r.claim(state, ctx, hl).is_some());

        winner.and_then(|i| self.0[i].hit_col(state, ctx, hl, area, point))
    }

    /// Screen cell for a column within the row the chain claimed
    pub fn cell(
        &mut self,
        state: &mut S,
        ctx: &LineCtx,
        hl: &Highlight,
        area: Rect,
        col: usize,
    ) -> Option<Position> {
        let winner = self
            .0
            .iter_mut()
            .position(|r| r.claim(state, ctx, hl).is_some());

        match winner {
            Some(i) => self.0[i].cell(state, ctx, hl, area, col),
            None => plain_cell(ctx, area, col),
        }
    }
}

/// A grapheme of a plain drawn line
struct Grapheme {
    chr: usize,
    chars: usize,
    x: usize,
    width: usize,
}

/// Every grapheme of a line as `render_plain` lays it out
fn graphemes(ctx: &LineCtx) -> Vec<Grapheme> {
    let text = ctx.text.to_string().replace("\n", " ");
    let tab_width = ctx.tab.width();

    let mut out = Vec::new();
    let mut chr = 0;
    let mut x = 0;

    for g in text.graphemes(true) {
        let width = match g {
            "\t" => tab_width,
            // Control characters take no cells
            _ if g.contains(char::is_control) => 0,
            _ => g.width(),
        };
        let chars = g.chars().count();

        out.push(Grapheme {
            chr,
            chars,
            x,
            width,
        });

        chr += chars;
        x += width;
    }

    out
}

/// Screen cell for a column of a line drawn by `render_plain`, None when it does not fit
pub fn plain_cell(ctx: &LineCtx, area: Rect, col: usize) -> Option<Position> {
    if col > ctx.text.len_chars() {
        return None;
    }

    let cells = graphemes(ctx);
    let (x, width) = cells
        .iter()
        .find(|g| col < g.chr + g.chars)
        .map(|g| (g.x, g.width))
        .unwrap_or_else(|| (cells.last().map_or(0, |g| g.x + g.width), 1));

    if x + width.max(1) > area.width as usize {
        return None;
    }

    Some(Position::new(area.x + x as u16, area.y))
}

/// Column of a line drawn by `render_plain` under a point
pub fn plain_col(ctx: &LineCtx, area: Rect, point: Position) -> usize {
    let x = point.x.saturating_sub(area.x) as usize;
    let cells = graphemes(ctx);

    if let Some(g) = cells.iter().find(|g| g.width > 0 && x < g.x + g.width) {
        return g.chr;
    }

    let end = cells.last().map_or(0, |g| g.x + g.width);
    ctx.text.len_chars() + x.saturating_sub(end)
}

/// Draws a line as plain text
///
/// # Args
/// - `conceal` - A list of char-ranges to ignore
/// - `replace` - A list of left-to-right text replacements to do
pub fn render_plain(
    ctx: &LineCtx,
    hl: &Highlight,
    conceal: &[Range<usize>],
    replace: &[(&str, &str)],
    area: Rect,
    buf: &mut Buffer,
) {
    let len = ctx.text.len_chars();

    if len == 0 {
        let spans = vec![RSpan::styled(" ".to_owned(), hl.style_at(0))];

        Line::from(spans).render(area, buf);
        return;
    }

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

    let mut spans = Vec::new();

    for pair in points.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        if end <= start {
            continue;
        }

        if conceal
            .iter()
            .any(|range| range.start <= start && end <= range.end)
        {
            continue;
        }

        let mut text = ctx.text.slice(start..end).to_string().replace("\n", " ");
        for r in replace {
            text = text.replace(r.0, r.1);
        }

        spans.push(RSpan::styled(text, hl.style_at(start)));
    }

    let line_start = ctx.rope.line_to_char(ctx.line);
    let at_true_eof = line_start + len == ctx.rope.len_chars();
    if at_true_eof && hl.spans.iter().any(|span| span.range.start == len) {
        spans.push(RSpan::styled(" ".to_owned(), hl.style_at(len)));
    }

    Line::from(spans).render(area, buf);
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use ratatui::style::Style;
    use ropey::Rope;

    use super::*;
    use crate::StyleSpan;

    fn ctx(rope: &Rope) -> LineCtx<'_> {
        LineCtx {
            rope,
            line: 0,
            tab: Rc::from("   "),
            text: rope.line(0),
            width: 80,
            cursors: &[],
            primary: 0,
            tree: None,
        }
    }

    struct Fixed(Option<u16>);

    impl<S> Renderer<S> for Fixed {
        fn claim(&mut self, _state: &mut S, _ctx: &LineCtx, _hl: &Highlight) -> Option<u16> {
            self.0
        }

        fn render(
            &mut self,
            _state: &mut S,
            _ctx: &LineCtx,
            _hl: &Highlight,
            _area: Rect,
            _buf: &mut Buffer,
        ) {
        }
    }

    #[test]
    fn no_claim_measures_as_one_row() {
        let rope = Rope::from_str("hello");
        let mut chain = Renderers::default();

        chain.push(Fixed(None));

        assert_eq!(
            chain.measure(&mut (), &ctx(&rope), &Highlight::default()),
            1
        );
    }

    #[test]
    fn first_claim_wins_the_measure() {
        let rope = Rope::from_str("hello");
        let mut chain = Renderers::default();

        chain.push(Fixed(None));
        chain.push(Fixed(Some(4)));
        chain.push(Fixed(Some(9)));

        assert_eq!(
            chain.measure(&mut (), &ctx(&rope), &Highlight::default()),
            4
        );
    }

    #[test]
    fn fallback_renders_plain_styled_text() {
        let rope = Rope::from_str("hello");
        let line_ctx = ctx(&rope);
        let hl = Highlight::plain(vec![StyleSpan {
            range: 0..2,
            style: Style::new().fg(ratatui::style::Color::Red),
        }]);

        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);

        render_plain(&line_ctx, &hl, &[], &[], area, &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "h");
        assert_eq!(buf[(0, 0)].style().fg, Some(ratatui::style::Color::Red));
        assert_eq!(buf[(2, 0)].symbol(), "l");
        assert_eq!(buf[(2, 0)].style().fg, Some(ratatui::style::Color::Reset));
    }

    #[test]
    fn virtual_eof_span_renders_as_trailing_space() {
        let rope = Rope::from_str("abc");
        let line_ctx = ctx(&rope);
        let hl = Highlight::plain(vec![StyleSpan {
            range: 3..4,
            style: Style::new().bg(ratatui::style::Color::Green),
        }]);

        let area = Rect::new(0, 0, 4, 1);
        let mut buf = Buffer::empty(area);

        render_plain(&line_ctx, &hl, &[], &[], area, &mut buf);

        assert_eq!(buf[(2, 0)].symbol(), "c");
        assert_eq!(buf[(3, 0)].symbol(), " ");
        assert_eq!(buf[(3, 0)].style().bg, Some(ratatui::style::Color::Green));
    }

    #[test]
    fn conceal_ranges_hide_text() {
        let rope = Rope::from_str("**bold**");
        let line_ctx = ctx(&rope);
        let hl = Highlight::plain(vec![StyleSpan {
            range: 2..6,
            style: Style::new().fg(ratatui::style::Color::Red),
        }]);

        let area = Rect::new(0, 0, 8, 1);
        let mut buf = Buffer::empty(area);

        render_plain(&line_ctx, &hl, &[0..2, 6..8], &[], area, &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "b");
        assert_eq!(buf[(0, 0)].style().fg, Some(ratatui::style::Color::Red));
        assert_eq!(buf[(3, 0)].symbol(), "d");
        assert_eq!(buf[(4, 0)].symbol(), " ");
    }

    #[test]
    fn overlapping_spans_patch_in_order() {
        let rope = Rope::from_str("hello");
        let line_ctx = ctx(&rope);
        let hl = Highlight::plain(vec![
            StyleSpan {
                range: 0..5,
                style: Style::new().fg(ratatui::style::Color::Red),
            },
            StyleSpan {
                range: 1..3,
                style: Style::new().fg(ratatui::style::Color::Blue),
            },
        ]);

        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);

        render_plain(&line_ctx, &hl, &[], &[], area, &mut buf);

        assert_eq!(buf[(0, 0)].style().fg, Some(ratatui::style::Color::Red));
        assert_eq!(buf[(1, 0)].style().fg, Some(ratatui::style::Color::Blue));
        assert_eq!(buf[(2, 0)].style().fg, Some(ratatui::style::Color::Blue));
        assert_eq!(buf[(3, 0)].style().fg, Some(ratatui::style::Color::Red));
    }

    #[test]
    fn plain_cell_counts_wide_characters() {
        let rope = Rope::from_str("let 名 = x;");
        let line_ctx = ctx(&rope);
        let area = Rect::new(4, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        render_plain(&line_ctx, &Highlight::default(), &[], &[], area, &mut buf);

        assert_eq!(plain_cell(&line_ctx, area, 6), Some(Position::new(11, 0)));
        assert_eq!(buf[(11, 0)].symbol(), "=");
    }

    #[test]
    fn tabs_take_the_width_of_the_tab_string() {
        let rope = Rope::from_str("\tab");
        let line_ctx = ctx(&rope);
        let area = Rect::new(0, 0, 6, 1);
        let mut buf = Buffer::empty(area);
        let hl = Highlight::plain(vec![StyleSpan {
            range: 0..1,
            style: Style::new().bg(ratatui::style::Color::Green),
        }]);

        render_plain(&line_ctx, &hl, &[], &[("\t", &line_ctx.tab)], area, &mut buf);

        assert_eq!(plain_cell(&line_ctx, area, 0), Some(Position::new(0, 0)));
        assert_eq!(plain_cell(&line_ctx, area, 1), Some(Position::new(3, 0)));
        assert_eq!(plain_cell(&line_ctx, area, 2), Some(Position::new(4, 0)));
        assert_eq!(buf[(3, 0)].symbol(), "a");
        assert_eq!(buf[(4, 0)].symbol(), "b");

        for x in 0..3 {
            assert_eq!(buf[(x, 0)].style().bg, Some(ratatui::style::Color::Green));
        }
    }

    #[test]
    fn plain_col_inside_a_tab_is_the_tab() {
        let rope = Rope::from_str("\tab");
        let line_ctx = ctx(&rope);
        let area = Rect::new(0, 0, 6, 1);

        for x in 0..3 {
            assert_eq!(plain_col(&line_ctx, area, Position::new(x, 0)), 0);
        }
        assert_eq!(plain_col(&line_ctx, area, Position::new(3, 0)), 1);
    }

    #[test]
    fn plain_cell_past_the_edge_is_none() {
        let rope = Rope::from_str("hello");
        let line_ctx = ctx(&rope);
        let area = Rect::new(0, 0, 3, 1);

        assert_eq!(plain_cell(&line_ctx, area, 2), Some(Position::new(2, 0)));
        assert_eq!(plain_cell(&line_ctx, area, 4), None);
    }

    #[test]
    fn plain_cell_end_of_line_is_the_next_cell() {
        let rope = Rope::from_str("abc");
        let line_ctx = ctx(&rope);

        assert_eq!(
            plain_cell(&line_ctx, Rect::new(0, 0, 4, 1), 3),
            Some(Position::new(3, 0))
        );
        assert_eq!(plain_cell(&line_ctx, Rect::new(0, 0, 3, 1), 3), None);
        assert_eq!(plain_cell(&line_ctx, Rect::new(0, 0, 9, 1), 4), None);
    }

    #[test]
    fn plain_col_inverts_plain_cell() {
        let rope = Rope::from_str("let\t名 = x;");
        let line_ctx = ctx(&rope);
        let area = Rect::new(4, 0, 20, 1);

        for col in 0..line_ctx.text.len_chars() {
            let cell = plain_cell(&line_ctx, area, col).unwrap();
            assert_eq!(plain_col(&line_ctx, area, cell), col);
        }

        // The right half of the wide character
        assert_eq!(plain_col(&line_ctx, area, Position::new(11, 0)), 4);
    }

    #[test]
    fn plain_col_past_the_text_keeps_counting() {
        let rope = Rope::from_str("ab");
        let line_ctx = ctx(&rope);

        assert_eq!(
            plain_col(&line_ctx, Rect::new(0, 0, 10, 1), Position::new(5, 0)),
            5
        );
    }
}
