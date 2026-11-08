use kui::{Node, div};
use kwark_render::Element;

use kwark_core::Editor;

/// Empty space some characters wide
pub struct Gap(pub u16);

impl<S: 'static, C> Element<Editor<S>, C> for Gap {
    fn view(&mut self, state: &mut Editor<S>, _ctx: &C) -> Option<Node<Editor<S>>> {
        let width = f32::from(self.0) * state.core.pipeline.metrics.char_width;
        let gap = div().w(width).h_full();

        Some(state.core.theme.get("bar.gap").apply(gap))
    }
}
