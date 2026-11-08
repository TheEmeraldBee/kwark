use kui::Color;
use kwark_core::prelude::render::*;
use kwark_core::prelude::*;
use kwark_theme::Style;

const SELECTION_BG: Color = Color::rgb(0x45475A);

#[derive(PartialEq, Eq, Debug)]
pub struct SelectionTag;

/// Highlights the text that every cursor selects, styled by the theme key `cursor.selection`
pub struct Selection;

impl<S: 'static> Highlighter<Editor<S>> for Selection {
    fn highlight(&mut self, state: &mut Editor<S>, ctx: &LineCtx) -> Option<Highlight> {
        let line_start = ctx.rope.line_to_char(ctx.line);
        let line_end = line_start + ctx.text.len_chars();

        let style = Style::new()
            .bg(SELECTION_BG)
            .patch(state.core.theme.get("cursor.selection"));

        let spans: Vec<StyleSpan> = ctx
            .cursors
            .iter()
            .filter(|cursor| cursor.start() != cursor.end())
            .filter_map(|cursor| {
                let start = cursor.start().max(line_start);
                let end = (cursor.end() + 1).min(line_end);

                (start < end).then(|| StyleSpan {
                    range: start - line_start..end - line_start,
                    style,
                })
            })
            .collect();

        (!spans.is_empty()).then(|| Highlight::tagged(spans, SelectionTag))
    }
}
