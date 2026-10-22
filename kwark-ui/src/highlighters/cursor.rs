use kwark_core::prelude::{text::CursorOptions, *};
use ratatui::style::{Color, Modifier, Style};
use render::*;

#[derive(PartialEq, Eq, Debug)]
pub struct CursorTag;

/// Highlights the character under every cursor in the buffer
pub struct Cursor;

impl Highlighter<State> for Cursor {
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

            let on_virtual_eof = len > 0
                && caret == line_end
                && line_end == ctx.rope.len_chars()
                && ctx.text.char(len - 1) != '\n';

            if (caret >= line_start && caret < line_end) || on_empty_line || on_virtual_eof {
                let col = caret - line_start;

                spans.push(StyleSpan {
                    range: col..(col + 1),
                    style: caret_style,
                });
            }

            let sel_start = cursor.start().max(line_start);
            let sel_end = (cursor.end() + 1).min(line_end);

            if cursor.start() != cursor.end() && sel_start < sel_end {
                spans.push(StyleSpan {
                    range: sel_start - line_start..sel_end - line_start,
                    style: Style::default().bg(Color::Green),
                });
            }
        }

        if spans.is_empty() {
            None
        } else {
            Some(Highlight::tagged(spans, CursorTag))
        }
    }
}
