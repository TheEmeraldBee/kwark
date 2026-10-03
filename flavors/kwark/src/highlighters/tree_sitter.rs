use std::ops::Range;

use kwark::prelude::*;
use ratatui::style::Style;
use text_render::*;
use tree_sitter::*;

struct TSHighlight {
    range: Range<usize>,
    style: Style,
}

impl Ranged for TSHighlight {
    fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}

pub struct TSHighlighter;

impl Highlighter<State> for TSHighlighter {
    fn highlight(&mut self, state: &mut State, ctx: &LineCtx) -> Option<Highlight> {
        let (theme, store) = state.get::<(&Theme, &mut LanguageStore)>();

        let tree = ctx.tree?;

        let line_start = ctx.rope.line_to_byte(ctx.line);
        let line_end = line_start + ctx.text.len_bytes();

        let highlights = tree
            .query(store, ctx.rope, "highlights")
            .unwrap()
            .in_range(line_start..line_end)
            .flat_map(|m| {
                m.iter_captures()
                    .map(|x| TSHighlight {
                        range: x.1.byte_range(),
                        style: theme.get(&format!("ts.{}", x.0)),
                    })
                    .collect::<Vec<_>>()
            })
            .combine(|a, b| {
                let combined_style = combine([a.style, b.style]);
                TSHighlight {
                    range: a.range,
                    style: combined_style,
                }
            })
            .finish();

        let col_start = ctx.rope.byte_to_char(line_start);

        let spans = highlights
            .into_iter()
            .filter_map(|h| {
                let start = h.range.start.max(line_start);
                let end = h.range.end.min(line_end);

                if start >= end {
                    return None;
                }

                Some(StyleSpan {
                    range: (ctx.rope.byte_to_char(start) - col_start)
                        ..(ctx.rope.byte_to_char(end) - col_start),
                    style: h.style,
                })
            })
            .collect::<Vec<_>>();

        Some(Highlight::plain(spans))
    }
}
