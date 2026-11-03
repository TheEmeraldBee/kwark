use kwark_render::Border;
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use kwark_core::Editor;
use kwark_window::WindowCtx;

pub struct Simple(pub BorderType);

impl<S: 'static> Border<Editor<S>, WindowCtx> for Simple {
    fn render(
        &mut self,
        state: &mut Editor<S>,
        ctx: &WindowCtx,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) -> ratatui::prelude::Rect {
        let theme = &state.core.theme;
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
