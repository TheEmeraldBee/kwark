use kwark_text_buffer_renderer::Element;
use kwark_theme::Theme;
use ratatui::widgets::{Paragraph, Widget};

use crate::State;

pub struct Gap(pub u16);

impl<C> Element<State, C> for Gap {
    fn width(&mut self, _state: &mut State, _ctx: &C, _available: u16) -> u16 {
        self.0
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
        let theme = state.get::<&Theme>().get("bar.gap");
        Paragraph::new(" ".repeat(self.0 as usize))
            .style(theme)
            .render(area, buf);
    }
}
