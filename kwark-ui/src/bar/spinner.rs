use std::time::{Duration, Instant};

use kwark_render::Element;
use kwark_theme::Theme;
use ratatui::widgets::{Paragraph, Widget};

use kwark_core::{State, frame::Redraw, tasks::Tasks};

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

impl<C> Element<State, C> for Spinner {
    fn width(&mut self, state: &mut State, _ctx: &C, _available: u16) -> u16 {
        match state.get::<&Tasks>().busy() {
            true => 2,
            false => 0,
        }
    }

    fn render(
        &mut self,
        state: &mut State,
        _ctx: &C,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        if !state.get::<&Tasks>().busy() {
            return;
        }

        let step = (self.start.elapsed().as_millis() / STEP.as_millis()) as u32;
        let next = self.start + STEP * (step + 1);

        state.get::<&mut Redraw>().request_at(next);

        let style = state.get::<&Theme>().get("bar.spinner");
        Paragraph::new(format!("{} ", GLYPHS[step as usize % GLYPHS.len()]))
            .style(style)
            .render(area, buf);
    }
}
