use kwark_render::Element;
use ratatui::widgets::{Paragraph, Widget};

use kwark_core::Editor;

pub struct Gap(pub u16);

impl<S: 'static, C> Element<Editor<S>, C> for Gap {
    fn width(&mut self, _state: &mut Editor<S>, _ctx: &C, _available: u16) -> u16 {
        self.0
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
        let theme = state.core.theme.get("bar.gap");
        Paragraph::new(" ".repeat(self.0 as usize))
            .style(theme)
            .render(area, buf);
    }
}
