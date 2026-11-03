use kwark_render::Element;
use ratatui::widgets::{Paragraph, Widget};

use kwark_core::{Editor, Has};

use super::Plugin;

/// The command line text, or the latest log message
pub struct StatusLine;

impl<S: Has<Plugin> + 'static, C> Element<Editor<S>, C> for StatusLine {
    fn width(&mut self, _state: &mut Editor<S>, _ctx: &C, available: u16) -> u16 {
        available / 2
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
        let (core, command_line) = state.split::<Plugin>();
        let theme = &core.theme;

        let (style, text) = if core.input.mode() == "command" {
            core.log.clear();
            (
                theme.get("statusline.command"),
                format!(":{}", command_line.text()),
            )
        } else if let Some((message, severity)) = core.log.get().cloned() {
            (theme.get(&format!("statusline.log.{}", severity)), message)
        } else {
            (theme.get("statusline.empty"), "".to_string())
        };

        Paragraph::new(text).style(style).render(area, buf);
    }
}
