use std::time::{Duration, Instant};

use kwark_render::Element;
use ratatui::widgets::{Paragraph, Widget};

use kwark_core::Editor;

const GLYPHS: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const STEP: Duration = Duration::from_millis(80);

/// A spinner shown while background tasks run
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
    fn width(&mut self, state: &mut Editor<S>, _ctx: &C, _available: u16) -> u16 {
        match state.core.tasks.busy() {
            true => 1,
            false => 1,
        }
    }

    fn render(
        &mut self,
        state: &mut Editor<S>,
        _ctx: &C,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let step = (self.start.elapsed().as_millis() / STEP.as_millis()) as u32;

        if !state.core.tasks.busy() {
            let style = state.core.theme.get("bar.spinner");
            Paragraph::new(format!("{} ", GLYPHS[step as usize % GLYPHS.len()]))
                .style(style)
                .render(area, buf);
            return;
        }

        let next = self.start + STEP * (step + 1);

        state.core.redraw.request_at(next);

        let style = state.core.theme.get("bar.spinner");
        Paragraph::new(format!("{} ", GLYPHS[step as usize % GLYPHS.len()]))
            .style(style)
            .render(area, buf);
    }
}
