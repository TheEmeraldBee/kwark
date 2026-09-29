use kwark_text_buffer::Cursor;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ropey::Rope;

use crate::{Highlighters, LineCtx, Renderers};

/// A full set of storage for Renderers and Highlighters
pub struct Pipeline<S> {
    pub highlighters: Highlighters<S>,
    pub renderers: Renderers<S>,
}

impl<S> Default for Pipeline<S> {
    fn default() -> Self {
        Self {
            highlighters: Highlighters::default(),
            renderers: Renderers::default(),
        }
    }
}

impl<S> Pipeline<S> {
    /// Renders buffer lines from scroll_top downward, stopping once area is full
    pub fn render(
        &mut self,
        rope: &Rope,
        state: &mut S,
        scroll_top: usize,
        cursors: &[Cursor],
        primary: usize,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let mut resolved = Vec::new();
        let mut used: u16 = 0;
        let mut line = scroll_top;

        while used < area.height && line < rope.len_lines() {
            let ctx = LineCtx {
                rope,
                line,
                text: rope.line(line),
                width: area.width,
                cursors,
                primary,
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
                x: area.x,
                y,
                width: area.width,
                height: *height,
            };

            self.renderers.render(state, ctx, hl, row, buf);
            y += height;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_pipeline_renders_plain_rows() {
        let rope = Rope::from_str("hello\nworld\n");
        let area = Rect::new(0, 0, 5, 2);
        let mut buf = Buffer::empty(area);

        Pipeline::default().render(&rope, &mut (), 0, &[], 0, area, &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "h");
        assert_eq!(buf[(0, 1)].symbol(), "w");
    }

    #[test]
    fn stops_once_the_area_is_full() {
        let rope = Rope::from_str("a\nb\nc\nd\n");
        let area = Rect::new(0, 0, 1, 2);
        let mut buf = Buffer::empty(area);

        Pipeline::default().render(&rope, &mut (), 0, &[], 0, area, &mut buf);

        assert_eq!(buf[(0, 0)].symbol(), "a");
        assert_eq!(buf[(0, 1)].symbol(), "b");
    }
}
