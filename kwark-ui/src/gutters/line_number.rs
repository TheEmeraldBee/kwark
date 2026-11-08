use kui::{Painter, Point, Rect, Span};
use kwark_render::{Gutter, Highlight, LineCtx, Metrics};

use kwark_core::Editor;

pub struct LineNumber {
    /// Width in characters
    pub max_width: u16,
    pub relative: bool,
}

impl Default for LineNumber {
    fn default() -> Self {
        Self {
            max_width: 3,
            relative: false,
        }
    }
}

impl<S: 'static> Gutter<Editor<S>> for LineNumber {
    fn width(&self, metrics: &Metrics) -> f32 {
        f32::from(self.max_width) * metrics.char_width
    }

    fn render(
        &mut self,
        state: &mut Editor<S>,
        ctx: &LineCtx,
        _hl: &Highlight,
        area: Rect,
        p: &mut Painter,
    ) {
        let style = state.core.theme.get("gutter.line_number");

        let line_num = if self.relative {
            let caret = ctx.cursors[ctx.primary].caret();
            let caret_line = ctx.rope.char_to_line(caret);

            ctx.line.abs_diff(caret_line)
        } else {
            ctx.line + 1
        };

        let number = line_num.to_string();
        let layout = ctx
            .metrics
            .fonts
            .line(&[Span::new(&number, style.text(&ctx.metrics.text))]);

        p.clip(area, |p| {
            p.text(&layout, Point::new(area.left(), area.top()))
        });
    }
}
