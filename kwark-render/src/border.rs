use kui::{Edges, Painter, Rect};

/// Marks which sides of a window sit against another pane
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Boundaries {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

/// Draws a window's frame
pub trait Border<S, C> {
    /// Returns the space the frame takes on each side of the window
    fn insets(&mut self, state: &mut S, ctx: &C) -> Edges;

    /// Draws the frame over the whole window rect
    fn render(&mut self, state: &mut S, ctx: &C, area: Rect, p: &mut Painter);
}
