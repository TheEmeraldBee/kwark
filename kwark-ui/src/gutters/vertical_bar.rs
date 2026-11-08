use kui::{Painter, Rect};
use kwark_render::{Gutter, Highlight, LineCtx, Metrics};

use kwark_core::Editor;

/// Fallback line color for a theme without a gutter bar style
const FALLBACK: kui::Color = kui::Color::rgb(0x6C7086);

/// A thin line at the right of a gutter one character wide
pub struct VerticalBar;

impl<S: 'static> Gutter<Editor<S>> for VerticalBar {
    fn width(&self, metrics: &Metrics) -> f32 {
        metrics.char_width
    }

    fn render(
        &mut self,
        state: &mut Editor<S>,
        _ctx: &LineCtx,
        _hl: &Highlight,
        area: Rect,
        p: &mut Painter,
    ) {
        let color = state.core.theme.get("gutter.bar").fg.unwrap_or(FALLBACK);
        let x = area.left() + area.size.width / 2.0;

        p.fill(Rect::new(x, area.top(), 1.0, area.size.height), color);
    }
}
