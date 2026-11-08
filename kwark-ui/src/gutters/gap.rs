use kui::{Painter, Rect};
use kwark_render::{Gutter, Highlight, LineCtx, Metrics};

use kwark_core::Editor;

/// Empty space some characters wide
pub struct Gap(pub u16);

impl Default for Gap {
    fn default() -> Self {
        Self(1)
    }
}

impl<S: 'static> Gutter<Editor<S>> for Gap {
    fn width(&self, metrics: &Metrics) -> f32 {
        f32::from(self.0) * metrics.char_width
    }

    fn render(
        &mut self,
        state: &mut Editor<S>,
        _ctx: &LineCtx,
        _hl: &Highlight,
        area: Rect,
        p: &mut Painter,
    ) {
        if let Some(bg) = state.core.theme.get("gutter.gap").bg {
            p.fill(area, bg);
        }
    }
}
