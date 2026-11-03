use std::time::{Duration, Instant};

use chrono::{Local, Timelike};
use kwark_render::Element;
use ratatui::{
    buffer::CellWidth,
    widgets::{Paragraph, Widget},
};

use kwark_core::Editor;

/// Format specifiers that change every second
const SECOND_SPECIFIERS: [&str; 6] = ["%S", "%T", "%X", "%r", "%s", "%+"];

pub struct Clock {
    format: String,
    seconds: bool,

    next_text: String,
}

impl Clock {
    pub fn new(format: impl Into<String>) -> Self {
        let format = format.into();
        let seconds = SECOND_SPECIFIERS.iter().any(|s| format.contains(s));

        Self {
            format,
            seconds,

            next_text: String::new(),
        }
    }

    /// Time left until the displayed text changes
    fn until_next_tick(&self) -> Duration {
        let now = Local::now();
        let millis = now.timestamp_subsec_millis().min(999) as u64;

        match self.seconds {
            true => Duration::from_millis(1000 - millis),
            false => Duration::from_millis(60_000 - (now.second() as u64 * 1000 + millis)),
        }
    }
}

impl<S: 'static, C> Element<Editor<S>, C> for Clock {
    fn width(&mut self, _state: &mut Editor<S>, _ctx: &C, _available: u16) -> u16 {
        let render = Local::now().format(&self.format).to_string();

        let width = render.cell_width();

        self.next_text = render;

        width
    }

    fn height(&mut self, _state: &mut Editor<S>, _ctx: &C) -> u16 {
        1
    }

    fn render(
        &mut self,
        state: &mut Editor<S>,
        _ctx: &C,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        state
            .core
            .redraw
            .request_at(Instant::now() + self.until_next_tick());

        let style = state.core.theme.get("bar.clock");
        Paragraph::new(self.next_text.as_str())
            .style(style)
            .render(area, buf);
    }
}
