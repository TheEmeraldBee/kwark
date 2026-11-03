use kwark_render::Gutter;
use ratatui::widgets::{Paragraph, Widget};

use kwark_core::Editor;

pub struct Gap(pub u16);

impl Default for Gap {
    fn default() -> Self {
        Self(1)
    }
}

impl<S: 'static> Gutter<Editor<S>> for Gap {
    fn width(&self) -> u16 {
        self.0
    }

    fn render(
        &mut self,
        state: &mut Editor<S>,
        _ctx: &kwark_render::LineCtx,
        _hl: &kwark_render::Highlight,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let style = state.core.theme.get("gutter.gap");
        Paragraph::new("  ".repeat(self.0 as usize))
            .style(style)
            .render(area, buf);
    }
}
