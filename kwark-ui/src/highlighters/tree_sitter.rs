use std::ops::Range;

use kwark_core::State;
use kwark_render::*;
use kwark_theme::Theme;
use kwark_tree_sitter::{query::Ranged, store::LanguageStore};
use ratatui::style::Style;

struct TreeSitterHighlight {
    range: Range<usize>,
    style: Style,
    priority: usize,
}

impl Ranged for TreeSitterHighlight {
    fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    fn priority(&self) -> usize {
        self.priority
    }
}

pub struct TreeSitter;

impl Highlighter<State> for TreeSitter {
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
                let priority = m.pattern_index();
                m.iter_captures()
                    .map(|x| TreeSitterHighlight {
                        range: x.1.byte_range(),
                        style: theme.get(&format!("ts.{}", x.0)),
                        priority,
                    })
                    .collect::<Vec<_>>()
            })
            .combine(|a, b| {
                let combined_style = combine([a.style, b.style]);
                let range = a.range.start.min(b.range.start)..a.range.end.max(b.range.end);
                TreeSitterHighlight {
                    range,
                    style: combined_style,
                    priority: a.priority.max(b.priority),
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
