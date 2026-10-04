use kwark_text_buffer::Cursor;
use kwark_tree_sitter::tree::SyntaxTree;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ropey::Rope;

use crate::{Highlighters, LineCtx, Renderers};

/// A full set of storage for Renderers and Highlighters
pub struct Pipeline<S> {
    pub highlighters: Highlighters<S>,
    pub renderers: Renderers<S>,
}

impl<S> Default for Pipeline<S> {
    fn default() -> Self {
        Self {
            highlighters: Highlighters::default(),
            renderers: Renderers::default(),
        }
    }
}

pub struct RenderArgs<'a, S> {
    pub rope: &'a Rope,
    pub state: &'a mut S,
    pub scroll_top: usize,
    pub cursors: &'a [Cursor],
    pub primary: usize,
    pub tree: Option<&'a SyntaxTree>,
    pub area: Rect,
    pub buf: &'a mut Buffer,
}

impl<S> Pipeline<S> {
    /// Renders buffer lines from scroll_top downward, stopping once area is full
    pub fn render(
        &mut self,
        RenderArgs {
            rope,
            state,
            scroll_top,
            cursors,
            primary,
            tree,
            area,
            buf,
        }: RenderArgs<S>,
    ) {
        let mut resolved = Vec::new();
        let mut used: u16 = 0;
        let mut line = scroll_top;

        while used < area.height && line < rope.len_lines() {
            let ctx = LineCtx {
                rope,
                line,
                text: rope.line(line),
                width: area.width,
                cursors,
                primary,
                tree,
            };

            let hl = self.highlighters.resolve(state, &ctx);
            let height = self
                .renderers
                .measure(state, &ctx, &hl)
                .min(area.height - used);

            used += height;
            resolved.push((ctx, hl, height));
            line += 1;
        }

        let mut y = area.y;

        for (ctx, hl, height) in &resolved {
            let row = Rect {
                x: area.x,
                y,
                width: area.width,
                height: *height,
            };

            self.renderers.render(state, ctx, hl, row, buf);
            y += height;
        }
    }
}
