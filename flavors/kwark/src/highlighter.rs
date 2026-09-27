use kwark::prelude::*;
use ratatui::{
    style::{Modifier, Style},
    widgets::{Paragraph, Widget},
};
use text_render::*;

/// Highlights the character under every cursor in the buffer
pub struct CursorHighlighter;

impl Highlighter<State> for CursorHighlighter {
    fn highlight(&mut self, _state: &mut State, ctx: &LineCtx) -> Option<Highlight> {
        let line_start = ctx.rope.line_to_char(ctx.line);
        let len = ctx.text.len_chars();

        let spans: Vec<StyleSpan> = ctx
            .cursors
            .iter()
            .filter_map(|cursor| {
                let col = cursor.caret().checked_sub(line_start)?;
                if col > len {
                    return None;
                }

                let end = (col + 1).min(len);

                Some(StyleSpan {
                    range: col..end,
                    style: Style::default().add_modifier(Modifier::REVERSED),
                })
            })
            .collect();

        if spans.is_empty() {
            None
        } else {
            Some(Highlight::plain(spans))
        }
    }
}

/// Highlights all text the same color
pub struct ConstantHighlighter;

impl Highlighter<State> for ConstantHighlighter {
    fn highlight(&mut self, _state: &mut State, ctx: &LineCtx) -> Option<Highlight> {
        let span = StyleSpan {
            range: 0..ctx.text.len_chars(),
            style: Style::default().fg(ratatui::style::Color::Rgb(201, 140, 176)),
        };

        Some(Highlight::plain(vec![span]))
    }
}

pub struct HelloWorldRenderer;

impl Renderer<State> for HelloWorldRenderer {
    fn claim(&mut self, _state: &mut State, ctx: &LineCtx, _hl: &Highlight) -> Option<u16> {
        if ctx.text.to_string().contains("freda") {
            Some(1)
        } else {
            None
        }
    }

    fn render(
        &mut self,
        _state: &mut State,
        ctx: &LineCtx,
        hl: &Highlight,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let text = ctx.text.to_string();

        let text = text.replace("freda", "<3 Freda <3");

        Paragraph::new(text)
            .style(hl.spans[0].style)
            .render(area, buf);
    }
}
