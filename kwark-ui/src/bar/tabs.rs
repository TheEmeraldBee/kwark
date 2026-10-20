use kwark_render::Element;
use kwark_theme::Theme;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use kwark_core::State;
use kwark_window::WindowCtx;

/// Lists a window's open buffers, highlighting the primary one
pub struct Tabs;

impl Element<State, WindowCtx> for Tabs {
    fn width(&mut self, _state: &mut State, _ctx: &WindowCtx, available: u16) -> u16 {
        available
    }

    fn render(&mut self, state: &mut State, ctx: &WindowCtx, area: Rect, buf: &mut Buffer) {
        let theme = state.get::<&Theme>();

        let border_style = match ctx.active {
            true => theme.get("bar.window.selected"),
            false => theme.get("bar.window"),
        };

        let primary_tab_style = theme.get("bar.tab.primary");
        let tab_style = theme.get("bar.tab");

        let edge_span = Span::styled(" --- ", border_style);
        let mut spans = vec![edge_span.clone()];

        for tab in &ctx.tabs {
            if tab.primary {
                spans.push(Span::styled(tab.name.clone(), primary_tab_style));
            } else {
                spans.push(Span::styled(tab.name.clone(), tab_style));
            }

            spans.push(Span::styled(" - ", border_style));
        }

        if spans.len() > 1 {
            spans.pop();
        }

        spans.push(edge_span);

        Paragraph::new(Line::from_iter(spans))
            .centered()
            .render(area, buf);
    }
}
