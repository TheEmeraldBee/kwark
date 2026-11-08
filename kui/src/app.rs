use crate::{Color, Event, Fonts, Node, Size};

/// Options for the window that kui opens
#[derive(Clone, Debug)]
pub struct WindowOptions {
    pub title: String,
    /// Inner size in logical pixels
    pub size: Size,
    /// Color under everything the app draws
    pub background: Color,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "kui".into(),
            size: Size::new(1024.0, 768.0),
            background: Color::BLACK,
        }
    }
}

/// Access to the window from app code
pub struct Cx<'a> {
    pub(crate) fonts: &'a mut Fonts,
    pub(crate) size: Size,
    pub(crate) scale: f32,
    pub(crate) redraw: bool,
}

impl Cx<'_> {
    /// Returns the inner size of the window in logical pixels
    pub fn size(&self) -> Size {
        self.size
    }

    /// Returns the number of physical pixels in one logical pixel
    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn fonts(&mut self) -> &mut Fonts {
        self.fonts
    }

    /// Asks for a new frame
    pub fn request_redraw(&mut self) {
        self.redraw = true;
    }
}

/// An application that kui drives
pub trait App: 'static {
    /// Data that element handlers receive
    type State: 'static;

    /// Options for the window
    fn window(&self) -> WindowOptions {
        WindowOptions::default()
    }

    /// Returns the state that element handlers receive
    fn state(&mut self) -> &mut Self::State;

    /// Handles an input event that no element took
    fn event(&mut self, _event: Event, _cx: &mut Cx) {}

    /// Returns the element tree for one frame
    fn view(&mut self, cx: &mut Cx) -> Node<Self::State>;
}
