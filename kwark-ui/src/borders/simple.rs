use kwark_render::Border;
use kwark_theme::Theme;
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use kwark_core::State;
use kwark_window::WindowCtx;

pub struct Simple(pub BorderType);

impl Border<State, WindowCtx> for Simple {
    fn render(
        &mut self,
        state: &mut State,
        ctx: &WindowCtx,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) -> ratatui::prelude::Rect {
        let theme = state.get::<&Theme>();
        let style = match ctx.active {
            true => theme.get("borders.active"),
            false => theme.get("borders"),
        };

        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(self.0)
            .style(style);

        let out_area = block.inner(area);

        block.render(area, buf);

        out_area
    }
}
