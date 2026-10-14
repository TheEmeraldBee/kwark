use kwark_text_buffer_renderer::Border;
use kwark_theme::Theme;
use ratatui::widgets::{Block, Borders, Widget};

use crate::{State, layout::WindowCtx};

pub struct SimpleBorders;

impl Border<State, WindowCtx> for SimpleBorders {
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

        let block = Block::new().borders(Borders::ALL).style(style);

        let out_area = block.inner(area);

        block.render(area, buf);

        out_area
    }
}
