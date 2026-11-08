use std::rc::Rc;

use kui::{CaretStyle, Painter, Rect};

use crate::{Highlight, LineCtx};

/// One cursor's caret on a drawn line
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Caret {
    /// The char column within the line
    pub col: usize,
    /// The character cell the caret sits on
    pub rect: Rect,
    pub primary: bool,
}

/// What a caret belongs to, for picking its look
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaretCtx {
    /// Whether it is the primary cursor
    pub primary: bool,
    /// Whether its window or prompt has focus
    pub active: bool,
}

/// The look of every caret, in buffers and prompts
pub trait CaretStyler<S> {
    fn style(&self, state: &S, ctx: CaretCtx) -> CaretStyle;
}

/// The caret styler of a pipeline, if it has one, shared by every clone
pub struct Carets<S>(Option<Rc<dyn CaretStyler<S>>>);

impl<S> Default for Carets<S> {
    fn default() -> Self {
        Self(None)
    }
}

impl<S> Clone for Carets<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S> Carets<S> {
    /// Sets the styler, replacing any before it
    pub fn set(&mut self, styler: impl CaretStyler<S> + 'static) -> &mut Self {
        self.0 = Some(Rc::new(styler));
        self
    }

    /// Removes the styler, so that no carets draw
    pub fn clear(&mut self) -> &mut Self {
        self.0 = None;
        self
    }

    pub fn is_set(&self) -> bool {
        self.0.is_some()
    }

    /// Returns the look of a caret, or None when no carets draw
    pub fn style(&self, state: &S, ctx: CaretCtx) -> Option<CaretStyle> {
        self.0.as_ref().map(|styler| styler.style(state, ctx))
    }

    /// Draws the carets of a line over its text
    pub fn render(
        &self,
        state: &S,
        ctx: &LineCtx,
        hl: &Highlight,
        carets: &[Caret],
        p: &mut Painter,
    ) {
        for caret in carets {
            let look = CaretCtx {
                primary: caret.primary,
                active: ctx.active,
            };
            let Some(style) = self.style(state, look) else {
                return;
            };

            let under = ctx.text.get_char(caret.col).map(|c| {
                let text = hl.style_at(caret.col).text(&ctx.metrics.text);
                (c, text)
            });
            style.paint(caret.rect, under.as_ref().map(|(c, s)| (*c, s)), p);
        }
    }
}

/// Returns the char column of each cursor whose caret sits on the line, with whether it is the primary
pub fn line_carets(ctx: &LineCtx) -> Vec<(usize, bool)> {
    let line_start = ctx.rope.line_to_char(ctx.line);
    let len = ctx.text.len_chars();
    let line_end = line_start + len;

    ctx.cursors
        .iter()
        .enumerate()
        .filter_map(|(i, cursor)| {
            let caret = cursor.caret();

            let on_empty_line = len == 0 && caret == line_start;
            let on_virtual_eof = len > 0
                && caret == line_end
                && line_end == ctx.rope.len_chars()
                && ctx.text.char(len - 1) != '\n';

            let on_line =
                (line_start..line_end).contains(&caret) || on_empty_line || on_virtual_eof;
            on_line.then(|| (caret - line_start, i == ctx.primary))
        })
        .collect()
}
