use kwark::prelude::*;
use ratatui::style::{Modifier, Style};
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
