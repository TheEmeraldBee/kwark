use crossterm::event::KeyEvent;

/// A General Update that runs on a fixed interval
#[derive(Debug)]
pub struct Frame;

/// An event when a key-code event occurs
#[derive(Debug)]
pub struct Input(pub KeyEvent);
