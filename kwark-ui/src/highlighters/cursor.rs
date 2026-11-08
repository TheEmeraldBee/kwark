use kui::Color;
use kwark_core::prelude::render::*;
use kwark_core::prelude::*;
use kwark_theme::Style;

const CARET_BG: Color = Color::rgb(0xF5E0DC);
const CARET_FG: Color = Color::rgb(0x1E1E2E);
const ALL_BG: Color = Color::rgb(0x89B4FA);
const OTHER_BG: Color = Color::rgb(0x585B70);
const SELECTION_BG: Color = Color::rgb(0x45475A);

#[derive(PartialEq, Eq, Debug)]
pub struct CursorTag;

/// Highlights the character under every cursor in the buffer
pub struct Cursor;

impl<S: 'static> Highlighter<Editor<S>> for Cursor {
    fn highlight(&mut self, state: &mut Editor<S>, ctx: &LineCtx) -> Option<Highlight> {
        let is_all = state.core.cursor.is_all();

        let line_start = ctx.rope.line_to_char(ctx.line);
        let len = ctx.text.len_chars();
        let line_end = line_start + len;

        let mut spans = vec![];

        let themed = state.core.theme.get("cursor.primary");
        let primary = Style::new().bg(CARET_BG).fg(CARET_FG).patch(themed);
        let selection = Style::new()
            .bg(SELECTION_BG)
            .patch(state.core.theme.get("cursor.selection"));

        for (i, cursor) in ctx.cursors.iter().enumerate() {
            let caret = cursor.caret();

            let caret_style = if i == ctx.primary {
                primary
            } else if is_all {
                Style::new().bg(ALL_BG).fg(CARET_FG)
            } else {
                Style::new().bg(OTHER_BG)
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
                    style: selection,
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
