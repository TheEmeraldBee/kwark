use kwark_input::Chord;

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
