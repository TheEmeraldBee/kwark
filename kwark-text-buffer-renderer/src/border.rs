use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

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
    /// Draws the frame and returns the rect left over
    fn render(&mut self, state: &mut S, ctx: &C, area: Rect, buf: &mut Buffer) -> Rect;
}
