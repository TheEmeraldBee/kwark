use kwark_render::Element;
use ratatui::{
    buffer::CellWidth,
    widgets::{Paragraph, Widget},
};

use kwark_core::Editor;

pub struct Mode;

impl<S: 'static, C> Element<Editor<S>, C> for Mode {
    fn width(&mut self, state: &mut Editor<S>, _ctx: &C, available: u16) -> u16 {
        state.core.input.mode().cell_width() + 2.min(available)
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
        let (input, theme) = (&state.core.input, &state.core.theme);
        let mode = input.mode();
        let style = theme.get(&format!("mode.{mode}"));

        Paragraph::new(format!(" {} ", mode.to_ascii_uppercase()))
            .style(style)
            .render(area, buf);
    }
}
