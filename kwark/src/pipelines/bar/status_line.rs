use kwark_text_buffer_renderer::Element;
use kwark_theme::Theme;
use ratatui::widgets::{Paragraph, Widget};

use crate::{InputState, State, log::LogState, plugins::command_line::CommandLineState};

pub struct StatusLine;

impl<C> Element<State, C> for StatusLine {
    fn width(&mut self, _state: &mut State, _ctx: &C, available: u16) -> u16 {
        available
    }

    fn height(&mut self, _state: &mut State, _ctx: &C) -> u16 {
        1
    }

    fn render(
        &mut self,
        state: &mut State,
        _ctx: &C,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let mode = state.get::<&InputState>().mode().to_string();

        let (style, text) = if state.has::<CommandLineState>() && mode == "command" {
            state.get::<&mut LogState>().clear();
            (
                state.get::<&Theme>().get("statusline.command"),
                format!(":{}", state.get::<&CommandLineState>().text()),
            )
        } else if let Some((message, severity)) = state.get::<&LogState>().get().cloned() {
            (
                state
                    .get::<&Theme>()
                    .get(&format!("statusline.log.{}", severity)),
                message,
            )
        } else {
            (
                state.get::<&Theme>().get("statusline.empty"),
                "".to_string(),
            )
        };

        Paragraph::new(text).style(style).render(area, buf);
    }
}
