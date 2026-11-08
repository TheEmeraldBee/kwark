use kui::{Edges, Painter, Rect};
use kwark_render::Border;

use kwark_core::Editor;
use kwark_window::WindowCtx;

/// Lines only where a window meets another, each window drawing its half of the shared line
pub struct LowProfile(pub f32);

impl LowProfile {
    fn half(&self) -> f32 {
        self.0 / 2.0
    }
}

impl<S: 'static> Border<Editor<S>, WindowCtx> for LowProfile {
    fn insets(&mut self, _state: &mut Editor<S>, ctx: &WindowCtx) -> Edges {
        let side = |touches: bool| if touches { self.half() } else { 0.0 };
        let b = ctx.boundaries;

        Edges {
            top: side(b.top),
            right: side(b.right),
            bottom: side(b.bottom),
            left: side(b.left),
        }
    }

    fn render(&mut self, state: &mut Editor<S>, ctx: &WindowCtx, area: Rect, p: &mut Painter) {
        let color = super::line_color(state, ctx);
        let (half, b) = (self.half(), ctx.boundaries);
        let (w, h) = (area.size.width, area.size.height);

        if b.top {
            p.fill(Rect::new(area.left(), area.top(), w, half), color);
        }
        if b.bottom {
            p.fill(Rect::new(area.left(), area.bottom() - half, w, half), color);
        }
        if b.left {
            p.fill(Rect::new(area.left(), area.top(), half, h), color);
        }
        if b.right {
            p.fill(Rect::new(area.right() - half, area.top(), half, h), color);
        }
    }
}
