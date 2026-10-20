use kwark_render::Border;
use kwark_theme::Theme;
use ratatui::{
    symbols::merge::MergeStrategy,
    widgets::{Block, BorderType, Borders, Widget},
};

use kwark_core::State;
use kwark_window::WindowCtx;

pub struct LowProfile(pub BorderType);

impl Border<State, WindowCtx> for LowProfile {
    fn render(
        &mut self,
        state: &mut State,
        ctx: &WindowCtx,
        mut area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) -> ratatui::prelude::Rect {
        let theme = state.get::<&Theme>();
        let style = match ctx.active {
            true => theme.get("borders.active"),
            false => theme.get("borders"),
        };

        if ctx.boundaries.left {
            area.x -= 1;
            area.width += 1;
        }
        if ctx.boundaries.top {
            area.y -= 1;
            area.height += 1;
        }

        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(self.0)
            .merge_borders(MergeStrategy::Fuzzy)
            .style(style);

        let inner = block.inner(area);

        block.render(area, buf);

        inner
    }
}
