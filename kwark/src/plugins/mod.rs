mod internal_utils;

pub mod cursor_overrides;

/// A fully built out system for creating fast binds for handling fast, efficient cursor movement
pub mod movement;

/// A set of functions and systems to make building an "insert" mode easy
pub mod insert;

/// A set of bindings (and state) for using the mouse effectively
pub mod visual;

/// A set of commands and bindings for window, split, and buffer handling
pub mod window;
