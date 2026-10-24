use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// Something drawn over the whole frame after every window
pub trait Overlay<S> {
    /// Draws the overlay anywhere within `area`
    fn render(&mut self, state: &mut S, area: Rect, buf: &mut Buffer);
}

impl<S, F: FnMut(&mut S, Rect, &mut Buffer)> Overlay<S> for F {
    fn render(&mut self, state: &mut S, area: Rect, buf: &mut Buffer) {
        self(state, area, buf)
    }
}

/// A set of overlays drawn in insertion order
pub struct Overlays<S>(Vec<Box<dyn Overlay<S>>>);

impl<S> Default for Overlays<S> {
    fn default() -> Self {
        Self(vec![])
    }
}

impl<S> Overlays<S> {
    /// Appends an overlay, drawn above every overlay before it
    pub fn push(&mut self, o: impl Overlay<S> + 'static) -> &mut Self {
        self.0.push(Box::new(o));
        self
    }

    /// Draws every overlay to the buffer
    pub fn render(&mut self, state: &mut S, area: Rect, buf: &mut Buffer) {
        for o in &mut self.0 {
            o.render(state, area, buf);
        }
    }
}
