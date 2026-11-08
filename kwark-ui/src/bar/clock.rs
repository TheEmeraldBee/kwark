use std::time::{Duration, Instant};

use chrono::{Local, Timelike};
use kui::{Node, text};
use kwark_render::Element;

use kwark_core::Editor;

/// Format specifiers that change every second
const SECOND_SPECIFIERS: [&str; 6] = ["%S", "%T", "%X", "%r", "%s", "%+"];

pub struct Clock {
    format: String,
    seconds: bool,
}

impl Clock {
    pub fn new(format: impl Into<String>) -> Self {
        let format = format.into();
        let seconds = SECOND_SPECIFIERS.iter().any(|s| format.contains(s));

        Self { format, seconds }
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
    fn view(&mut self, state: &mut Editor<S>, _ctx: &C) -> Option<Node<Editor<S>>> {
        state
            .core
            .redraw
            .request_at(Instant::now() + self.until_next_tick());

        let now = Local::now().format(&self.format).to_string();
        Some(state.core.theme.get("bar.clock").apply(text(now)))
    }
}
