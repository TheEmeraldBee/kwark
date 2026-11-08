use kui::{Painter, Rect};

use crate::{Highlight, LineCtx, Metrics};

pub trait Gutter<S> {
    /// Returns the width of the gutter in logical pixels
    ///
    /// # IMPORTANT
    /// This should return the same width for every line
    fn width(&self, metrics: &Metrics) -> f32;

    /// Draws the gutter for a given line
    ///
    /// `area.size.width` is always [`Self::width`] and `area.size.height` is the line's measured height
    fn render(&mut self, state: &mut S, ctx: &LineCtx, hl: &Highlight, area: Rect, p: &mut Painter);
}

pub struct Gutters<S>(Vec<Box<dyn Gutter<S>>>);

impl<S> Default for Gutters<S> {
    fn default() -> Self {
        Self(vec![])
    }
}

impl<S> Gutters<S> {
    /// Adds a new gutter to the set
    pub fn push(&mut self, gutter: impl Gutter<S> + 'static) -> &mut Self {
        self.0.push(Box::new(gutter));
        self
    }

    /// Returns the total width of all gutters in logical pixels
    pub fn measure(&self, metrics: &Metrics) -> f32 {
        self.0.iter().map(|x| x.width(metrics)).sum()
    }

    /// Draws every gutter for the line, left to right within `area`
    pub fn render(
        &mut self,
        state: &mut S,
        ctx: &LineCtx,
        hl: &Highlight,
        area: Rect,
        p: &mut Painter,
    ) {
        let mut x = area.left();
        for gutter in &mut self.0 {
            let width = gutter.width(ctx.metrics);
            let rect = Rect::new(x, area.top(), width, area.size.height);
            x += width;

            gutter.render(state, ctx, hl, rect, p);
        }
    }
}
