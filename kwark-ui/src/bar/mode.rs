use kui::{Node, text};
use kwark_render::Element;

use kwark_core::Editor;

/// The current input mode
pub struct Mode;

impl<S: 'static, C> Element<Editor<S>, C> for Mode {
    fn view(&mut self, state: &mut Editor<S>, _ctx: &C) -> Option<Node<Editor<S>>> {
        let mode = state.core.input.mode();
        let style = state.core.theme.get(&format!("mode.{mode}"));
        let label = format!(" {} ", mode.to_ascii_uppercase());

        Some(style.apply(text(label)))
    }
}
