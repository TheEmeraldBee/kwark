use std::time::{Duration, Instant};

use kui::{Node, text};
use kwark_render::Element;

use kwark_core::Editor;

const GLYPHS: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const STEP: Duration = Duration::from_millis(80);

/// A spinner that turns while background tasks run
pub struct Spinner {
    start: Instant,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: 'static, C> Element<Editor<S>, C> for Spinner {
    fn view(&mut self, state: &mut Editor<S>, _ctx: &C) -> Option<Node<Editor<S>>> {
        let step = (self.start.elapsed().as_millis() / STEP.as_millis()) as u32;

        if state.core.tasks.busy() {
            state.core.redraw.request_at(self.start + STEP * (step + 1));
        }

        let glyph = format!("{} ", GLYPHS[step as usize % GLYPHS.len()]);
        Some(state.core.theme.get("bar.spinner").apply(text(glyph)))
    }
}
