use kwark::prelude::{text_buffer::CursorOptions, *};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span as RSpan},
    widgets::Widget,
};
use text_render::*;

/// Highlights the character under every cursor in the buffer
pub struct CursorHighlighter;

impl Highlighter<State> for CursorHighlighter {
    fn highlight(&mut self, state: &mut State, ctx: &LineCtx) -> Option<Highlight> {
        let is_all = state.get::<&CursorOptions>().is_all();

        let line_start = ctx.rope.line_to_char(ctx.line);
        let len = ctx.text.len_chars();
        let line_end = line_start + len;

        let mut spans = vec![];

        for (i, cursor) in ctx.cursors.iter().enumerate() {
            let caret = cursor.caret();

            let caret_style = if i == ctx.primary {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                if is_all {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default().bg(Color::DarkGray)
                }
            };

            let on_empty_line = len == 0 && caret == line_start;

            if (caret >= line_start && caret < line_end) || on_empty_line {
                let col = caret - line_start;

                spans.push(StyleSpan {
                    range: col..(col + 1),
                    style: caret_style,
                });
            }

            let sel_start = cursor.start().max(line_start);
            let sel_end = cursor.end().min(line_end);

            if sel_start < sel_end {
                spans.push(StyleSpan {
                    range: sel_start - line_start..sel_end - line_start,
                    style: Style::default().bg(Color::Green),
                });
            }
        }

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
        let mut spans = Vec::new();
        let mut rest = text.as_str();
        let mut consumed = 0;

        while let Some(pos) = rest.find("freda") {
            push_plain(&rest[..pos], consumed, hl, &mut spans);

            spans.push(RSpan::styled(" <3 ".to_owned(), pink()));

            for (i, ch) in "freda".chars().enumerate() {
                let style = hl.style_at(consumed + pos + i);
                spans.push(RSpan::styled(ch.to_string(), style));
            }

            spans.push(RSpan::styled(" <3 ".to_owned(), pink()));

            consumed += pos + "freda".len();
            rest = &rest[pos + "freda".len()..];
        }

        push_plain(rest, consumed, hl, &mut spans);

        Line::from(spans).render(area, buf);
    }
}

/// Returns the pink used for the hearts
fn pink() -> Style {
    Style::default()
        .fg(Color::Rgb(255, 105, 180))
        .bg(Color::LightMagenta)
}

/// Appends each character of segment with its resolved style
fn push_plain(segment: &str, offset: usize, hl: &Highlight, spans: &mut Vec<RSpan<'static>>) {
    for (i, ch) in segment.chars().enumerate() {
        spans.push(RSpan::styled(ch.to_string(), hl.style_at(offset + i)));
    }
}
