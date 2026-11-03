use kwark_render::Gutter;
use ratatui::widgets::{Block, Borders, Widget};

use kwark_core::Editor;

pub struct VerticalBar;

impl<S: 'static> Gutter<Editor<S>> for VerticalBar {
    fn width(&self) -> u16 {
        1
    }

    fn render(
        &mut self,
        state: &mut Editor<S>,
        _ctx: &kwark_render::LineCtx,
        _hl: &kwark_render::Highlight,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let style = state.core.theme.get("gutter.bar");
        Block::new()
            .borders(Borders::RIGHT)
            .style(style)
            .render(area, buf);
    }
}
