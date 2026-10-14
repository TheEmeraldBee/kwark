use kwark_text_buffer_renderer::Gutter;
use kwark_theme::Theme;
use ratatui::widgets::{Block, Borders, Widget};

use crate::State;

pub struct VerticalBar;

impl Gutter<State> for VerticalBar {
    fn width(&self) -> u16 {
        1
    }

    fn render(
        &mut self,
        state: &mut State,
        _ctx: &kwark_text_buffer_renderer::LineCtx,
        _hl: &kwark_text_buffer_renderer::Highlight,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let style = state.get::<&Theme>().get("gutter.bar");
        Block::new()
            .borders(Borders::RIGHT)
            .style(style)
            .render(area, buf);
    }
}
