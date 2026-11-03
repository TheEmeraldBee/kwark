use std::ops::Range;

use kwark_core::State;
use kwark_render::*;
use kwark_theme::Theme;
use kwark_tree_sitter::store::LanguageStore;
use ratatui::style::Style;

struct TreeSitterHighlight {
    range: Range<usize>,
    style: Style,
    priority: usize,
}

/// Orders highlights so that nested and higher-priority captures come last
fn sort_for_patching(highlights: &mut [TreeSitterHighlight]) {
    highlights.sort_by_key(|h| (std::cmp::Reverse(h.range.len()), h.priority));
}

pub struct TreeSitter;

impl Highlighter<State> for TreeSitter {
    fn highlight(&mut self, state: &mut State, ctx: &LineCtx) -> Option<Highlight> {
        let (theme, store) = state.get::<(&Theme, &mut LanguageStore)>();

        let tree = ctx.tree?;

        let line_start = ctx.rope.line_to_byte(ctx.line);
        let line_end = line_start + ctx.text.len_bytes();

        let mut highlights = tree
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
            .finish();

        sort_for_patching(&mut highlights);

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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn hl(range: Range<usize>, color: Color, priority: usize) -> TreeSitterHighlight {
        TreeSitterHighlight {
            range,
            style: Style::default().fg(color),
            priority,
        }
    }

    fn resolve(mut highlights: Vec<TreeSitterHighlight>) -> Highlight {
        sort_for_patching(&mut highlights);

        Highlight::plain(
            highlights
                .into_iter()
                .map(|h| StyleSpan {
                    range: h.range,
                    style: h.style,
                })
                .collect(),
        )
    }

    #[test]
    fn nested_capture_keeps_outer_style_around_it() {
        // An escape inside a string, with the string pattern declared later
        let hl = resolve(vec![hl(0..10, Color::Green, 5), hl(3..5, Color::Red, 2)]);

        assert_eq!(hl.style_at(1).fg, Some(Color::Green));
        assert_eq!(hl.style_at(3).fg, Some(Color::Red));
        assert_eq!(hl.style_at(7).fg, Some(Color::Green));
    }

    #[test]
    fn later_pattern_wins_on_the_same_node() {
        let hl = resolve(vec![hl(0..3, Color::Blue, 9), hl(0..3, Color::White, 1)]);

        assert_eq!(hl.style_at(1).fg, Some(Color::Blue));
    }

    #[test]
    fn neighbors_inside_a_wide_capture_stay_separate() {
        let hl = resolve(vec![
            hl(0..20, Color::Green, 1),
            hl(2..4, Color::Red, 3),
            hl(10..12, Color::White, 0),
        ]);

        assert_eq!(hl.style_at(2).fg, Some(Color::Red));
        assert_eq!(hl.style_at(6).fg, Some(Color::Green));
        assert_eq!(hl.style_at(10).fg, Some(Color::White));
        assert_eq!(hl.style_at(15).fg, Some(Color::Green));
    }
}
