use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span as RSpan};
use ratatui::widgets::Widget;

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
            None => render_plain(ctx, hl, area, buf),
        }
    }
}

/// Draws a line as plain text
fn render_plain(ctx: &LineCtx, hl: &Highlight, area: Rect, buf: &mut Buffer) {
    let len = ctx.text.len_chars();

    // An empty line still draws one space so the caret can show on it
    if len == 0 {
        let spans = vec![RSpan::styled(" ".to_owned(), hl.style_at(0))];

        Line::from(spans).render(area, buf);
        return;
    }

    let mut points: Vec<usize> = hl
        .spans
        .iter()
        .flat_map(|span| [span.range.start, span.range.end])
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

        let text = ctx.text.slice(start..end).to_string().replace("\n", " ");
        spans.push(RSpan::styled(text, hl.style_at(start)));
    }

    Line::from(spans).render(area, buf);
}

#[cfg(test)]
mod test {
    use ratatui::style::Style;
    use ropey::Rope;

    use super::*;
    use crate::StyleSpan;

    fn ctx(rope: &Rope) -> LineCtx<'_> {
        LineCtx {
            rope,
            line: 0,
            text: rope.line(0),
            width: 80,
            cursors: &[],
            primary: 0,
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

        render_plain(&line_ctx, &hl, area, &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "h");
        assert_eq!(buf[(0, 0)].style().fg, Some(ratatui::style::Color::Red));
        assert_eq!(buf[(2, 0)].symbol(), "l");
        assert_eq!(buf[(2, 0)].style().fg, Some(ratatui::style::Color::Reset));
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

        render_plain(&line_ctx, &hl, area, &mut buf);

        assert_eq!(buf[(0, 0)].style().fg, Some(ratatui::style::Color::Red));
        assert_eq!(buf[(1, 0)].style().fg, Some(ratatui::style::Color::Blue));
        assert_eq!(buf[(2, 0)].style().fg, Some(ratatui::style::Color::Blue));
        assert_eq!(buf[(3, 0)].style().fg, Some(ratatui::style::Color::Red));
    }
}
