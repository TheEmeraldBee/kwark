use std::any::{Any, TypeId};
use std::collections::HashSet;

use kwark::prelude::{render::Renderer, *};
use ratatui::prelude::*;

/// Boxes a line that starts with `#`
#[derive(Default)]
pub struct HeaderRenderer {
    blockers: HashSet<TypeId>,
}

/// How many columns the left border takes up in front of the text
const TEXT_INSET: u16 = 1;

impl HeaderRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stops this renderer from claiming a line whose highlight is tagged `T`
    pub fn block<T: Any + 'static>(mut self) -> Self {
        self.blockers.insert(TypeId::of::<T>());
        self
    }

    /// Whether the highlight's tag is one this renderer was told to skip
    fn is_blocked(&self, hl: &render::Highlight) -> bool {
        hl.tag
            .as_ref()
            .is_some_and(|tag| self.blockers.contains(&(**tag).type_id()))
    }
}

impl Renderer<State> for HeaderRenderer {
    fn claim(
        &mut self,
        _state: &mut State,
        ctx: &render::LineCtx,
        hl: &render::Highlight,
    ) -> Option<u16> {
        if self.is_blocked(hl) {
            return None;
        }

        if ctx.text.to_string().starts_with('#') {
            Some(3)
        } else {
            None
        }
    }

    fn render(
        &mut self,
        _state: &mut State,
        ctx: &render::LineCtx,
        hl: &render::Highlight,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let (top, text_row, bottom) = split_rows(area);

        if let Some(y) = top {
            render_border(area.x, y, area.width, ('┌', '─', '┐'), buf);
        }

        if let Some(y) = bottom {
            render_border(area.x, y, area.width, ('└', '─', '┘'), buf);
        }

        let mut prefix = ctx.text.to_string();
        prefix.remove(0);
        let prefix_len = ctx.text.len_chars() - prefix.trim_start().chars().count();

        if let Some(y) = text_row {
            let inset = inset(area.width);

            Line::raw(format!("│{}", " ".repeat(inset as usize)))
                .render(Rect::new(area.x, y, inset, 1), buf);

            let text_area = Rect::new(area.x + inset, y, area.width - inset, 1);

            #[allow(clippy::single_range_in_vec_init)]
            render::render_plain(ctx, hl, &[0..prefix_len], text_area, buf);
        }
    }

    fn hit_col(
        &mut self,
        _state: &mut State,
        ctx: &render::LineCtx,
        _hl: &render::Highlight,
        area: Rect,
        point: Position,
    ) -> Option<usize> {
        let (top, text_row, bottom) = split_rows(area);

        let mut prefix = ctx.text.to_string();
        prefix.remove(0);
        let prefix_len = ctx.text.len_chars() - prefix.trim_start().chars().count();

        if Some(point.y) == top {
            return Some(0);
        }

        if Some(point.y) == bottom {
            return Some(ctx.text.len_chars());
        }

        if Some(point.y) == text_row {
            let inset = inset(area.width);
            return Some(point.x.saturating_sub(area.x + inset + prefix_len as u16) as usize);
        }

        None
    }
}

/// How many columns the border eats into `width` before the text starts
fn inset(width: u16) -> u16 {
    TEXT_INSET.min(width)
}

/// Splits a claimed area into its border/text rows, `None` for any row the area is too short to show
fn split_rows(area: Rect) -> (Option<u16>, Option<u16>, Option<u16>) {
    match area.height {
        0 => (None, None, None),
        1 => (None, Some(area.y), None),
        2 => (Some(area.y), Some(area.y + 1), None),
        _ => (Some(area.y), Some(area.y + 1), Some(area.y + 2)),
    }
}

/// Draws one border row across `width`, with the given corner/fill characters
fn render_border(x: u16, y: u16, width: u16, chars: (char, char, char), buf: &mut Buffer) {
    let (left, fill, right) = chars;

    let line = match width {
        0 => return,
        1 => fill.to_string(),
        _ => {
            let mut s = String::with_capacity(width as usize);
            s.push(left);
            for _ in 0..width - 2 {
                s.push(fill);
            }
            s.push(right);
            s
        }
    };

    Line::raw(line).render(Rect::new(x, y, width, 1), buf);
}
