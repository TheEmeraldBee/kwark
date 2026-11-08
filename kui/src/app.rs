use crate::{Color, Size};

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

/// An application that kui drives
pub trait App: 'static {
    /// Options for the window
    fn window(&self) -> WindowOptions {
        WindowOptions::default()
    }
}
