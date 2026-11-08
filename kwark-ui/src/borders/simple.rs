use kui::{Edges, Painter, Rect};
use kwark_render::Border;

use kwark_core::Editor;
use kwark_window::WindowCtx;

/// An outline around the whole window
pub struct Simple {
    /// Line width in logical pixels
    pub width: f32,
    pub radius: f32,
    /// Space between the line and the content
    pub padding: f32,
}

impl Default for Simple {
    fn default() -> Self {
        Self {
            width: 1.0,
            radius: 6.0,
            padding: 4.0,
        }
    }
}

impl<S: 'static> Border<Editor<S>, WindowCtx> for Simple {
    fn insets(&mut self, _state: &mut Editor<S>, _ctx: &WindowCtx) -> Edges {
        Edges::all(self.width + self.padding)
    }

    fn render(&mut self, state: &mut Editor<S>, ctx: &WindowCtx, area: Rect, p: &mut Painter) {
        let color = super::line_color(state, ctx);
        p.stroke(area, self.radius, self.width, color);
    }
}
