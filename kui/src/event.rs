use crate::Point;

/// An input event, with positions in logical pixels
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    MouseDown {
        position: Point,
        button: MouseButton,
    },
    MouseUp {
        position: Point,
        button: MouseButton,
    },
    MouseMove {
        position: Point,
    },
    /// The pointer left the window
    MouseLeave,
    Scroll {
        position: Point,
        delta: ScrollDelta,
    },
}

/// A mouse button
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

/// How far a scroll moved
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollDelta {
    /// Lines and columns, from a mouse wheel
    Lines(Point),
    /// Logical pixels, from a touchpad
    Pixels(Point),
}

impl From<winit::event::MouseButton> for MouseButton {
    fn from(button: winit::event::MouseButton) -> Self {
        use winit::event::MouseButton as B;

        match button {
            B::Left => Self::Left,
            B::Right => Self::Right,
            B::Middle => Self::Middle,
            B::Back => Self::Back,
            B::Forward => Self::Forward,
            B::Other(n) => Self::Other(n),
        }
    }
}
