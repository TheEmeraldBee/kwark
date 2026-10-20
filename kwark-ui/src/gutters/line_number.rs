use kwark_render::Gutter;
use kwark_theme::Theme;
use ratatui::widgets::{Paragraph, Widget};

use kwark_core::State;

pub struct LineNumber {
    pub max_width: u16,
    pub relative: bool,
}

impl Default for LineNumber {
    fn default() -> Self {
        Self {
            max_width: 3,
            relative: false,
        }
    }
}

impl Gutter<State> for LineNumber {
    fn width(&self) -> u16 {
        self.max_width
    }

    fn render(
        &mut self,
        state: &mut State,
        ctx: &kwark_render::LineCtx,
        _hl: &kwark_render::Highlight,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let line_theme = state.get::<&Theme>().get("gutter.line_number");

        let line_num = if self.relative {
            let caret = ctx.cursors[ctx.primary].caret();
            let caret_line = ctx.rope.char_to_line(caret);

            ctx.line.abs_diff(caret_line)
        } else {
            ctx.line + 1
        };

        Paragraph::new(format!("{}", line_num))
            .style(line_theme)
            .render(area, buf);
    }
}
