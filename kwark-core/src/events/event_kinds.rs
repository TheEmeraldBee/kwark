use kwark_input::Chord;

pub use kwark_buffer::{BufferChanged, BufferClosed, BufferOpened};
pub use kwark_input::ModeChanged;

/// An event when a key-code event occurs
#[derive(Debug, Clone, Copy)]
pub struct Input {
    pub chord: Chord,
    pub block: bool,
}

/// An event when a key-code event occurs, and nothing handled it
#[derive(Debug, Clone, Copy)]
pub struct ExtraInput {
    pub chord: Chord,
}

/// An event when the left mouse button moves while held down
#[derive(Debug, Clone, Copy)]
pub struct MouseDrag;

/// An event when the left mouse button is released
#[derive(Debug, Clone, Copy)]
pub struct MouseUp;

/// An event when the window changes size, in logical pixels
#[derive(Debug, Clone, Copy)]
pub struct Resize {
    pub width: f32,
    pub height: f32,
}

/// An event before each frame draws
#[derive(Clone, Copy)]
pub struct Frame {
    pub info: crate::frame::FrameInfo,
}
