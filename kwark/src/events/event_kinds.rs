use crossterm::event::KeyEvent;

#[derive(Copy, Clone, Debug)]
pub struct Frame;

#[derive(Clone, Debug)]
pub struct Input(pub KeyEvent);
