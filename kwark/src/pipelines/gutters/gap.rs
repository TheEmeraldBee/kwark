use kwark_text_buffer_renderer::Gutter;
use kwark_theme::Theme;
use ratatui::widgets::{Paragraph, Widget};

use crate::State;

pub struct Gap(pub u16);

impl Default for Gap {
    fn default() -> Self {
        Self(1)
    }
}

impl Gutter<State> for Gap {
    fn width(&self) -> u16 {
        self.0
    }

    fn render(
        &mut self,
        state: &mut State,
        _ctx: &kwark_text_buffer_renderer::LineCtx,
        _hl: &kwark_text_buffer_renderer::Highlight,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let style = state.get::<&Theme>().get("gutter.gap");
        Paragraph::new("  ".repeat(self.0 as usize))
            .style(style)
            .render(area, buf);
    }
}
