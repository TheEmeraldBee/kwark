use chrono::Local;
use kwark_render::Element;
use kwark_theme::Theme;
use ratatui::{
    buffer::CellWidth,
    widgets::{Paragraph, Widget},
};

use kwark_core::State;

pub struct Clock {
    format: String,

    next_text: String,
}

impl Clock {
    pub fn new(format: impl Into<String>) -> Self {
        Self {
            format: format.into(),

            next_text: String::new(),
        }
    }
}

impl<C> Element<State, C> for Clock {
    fn width(&mut self, _state: &mut State, _ctx: &C, _available: u16) -> u16 {
        let render = Local::now().format(&self.format).to_string();

        let width = render.cell_width();

        self.next_text = render;

        width
    }

    fn height(&mut self, _state: &mut State, _ctx: &C) -> u16 {
        1
    }

    fn render(
        &mut self,
        state: &mut State,
        _ctx: &C,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let style = state.get::<&Theme>().get("bar.clock");
        Paragraph::new(self.next_text.as_str())
            .style(style)
            .render(area, buf);
    }
}
