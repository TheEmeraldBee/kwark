use kwark_input::Chord;

/// A General Update that runs on a fixed interval
#[derive(Debug, Clone, Copy)]
pub struct Frame;

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
