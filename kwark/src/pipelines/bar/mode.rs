use kwark_text_buffer_renderer::Element;
use kwark_theme::Theme;
use ratatui::{
    buffer::CellWidth,
    widgets::{Paragraph, Widget},
};

use crate::{InputState, State};

pub struct Mode;

impl<C> Element<State, C> for Mode {
    fn width(&mut self, state: &mut State, _ctx: &C, available: u16) -> u16 {
        state.get::<&mut InputState>().mode().cell_width() + 2.min(available)
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
        let (input, theme) = state.get::<(&InputState, &Theme)>();
        let mode = input.mode();
        let style = theme.get(&format!("mode.{mode}"));

        Paragraph::new(format!(" {} ", mode.to_ascii_uppercase()))
            .style(style)
            .render(area, buf);
    }
}
