use kui::{Node, Rect};

/// Something drawn over the whole frame after every window
pub trait Overlay<S> {
    /// Returns the tree for the overlay, placed anywhere within `area`, or None to show nothing
    fn view(&mut self, state: &mut S, area: Rect) -> Option<Node<S>>;
}

impl<S, F: FnMut(&mut S, Rect) -> Option<Node<S>>> Overlay<S> for F {
    fn view(&mut self, state: &mut S, area: Rect) -> Option<Node<S>> {
        self(state, area)
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

    /// Returns the trees of every overlay that shows something, bottom first
    pub fn view(&mut self, state: &mut S, area: Rect) -> Vec<Node<S>> {
        self.0
            .iter_mut()
            .filter_map(|o| o.view(state, area))
            .collect()
    }
}
