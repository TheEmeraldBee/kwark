use kwark_core::State;
use kwark_render::Element;
use kwark_theme::Theme;
use ratatui::widgets::{Paragraph, Widget};

pub struct Freda;

const FREDA_TEXT: &str = "󱨝 Freda Shi 󰄛";

impl<C> Element<State, C> for Freda {
    fn width(&mut self, _state: &mut State, _ctx: &C, _available: u16) -> u16 {
        FREDA_TEXT.chars().count() as u16
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
        let style = state.get::<&Theme>().get("freda");

        Paragraph::new(FREDA_TEXT).style(style).render(area, buf);
    }
}
