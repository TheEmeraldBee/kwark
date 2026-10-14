use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::{Highlight, LineCtx};

pub trait Gutter<S> {
    /// Returns a requested width for the gutter
    ///
    /// # IMPORTANT
    /// This should **always** return a constant number
    fn width(&self) -> u16;

    /// Render the gutter content for a given line
    ///
    /// `area.width` will always be of width [`Self::width`]
    /// `area.height` with always be of height [`crate::Renderer::claim`], with a default of `1`
    fn render(
        &mut self,
        state: &mut S,
        ctx: &LineCtx,
        hl: &Highlight,
        area: Rect,
        buf: &mut Buffer,
    );
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

    /// Measures the total width of all gutters
    pub fn measure(&self) -> u16 {
        self.0.iter().map(|x| x.width()).sum()
    }

    /// Renders a full set of gutters to the line's buffer
    pub fn render(
        &mut self,
        state: &mut S,
        ctx: &LineCtx,
        hl: &Highlight,
        area: Rect,
        buf: &mut Buffer,
        renderer_height: u16,
    ) {
        let mut x = area.x;
        for gutter in &mut self.0 {
            let width = gutter.width();
            let rect = Rect::new(x, area.y, width, renderer_height);
            x += width;

            gutter.render(state, ctx, hl, rect, buf);
        }
    }
}
