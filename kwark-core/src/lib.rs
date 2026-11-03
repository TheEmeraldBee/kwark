mod editor;
mod flags;
mod plugin;
mod syntax;

pub mod commands;
pub mod events;
pub mod frame;
pub mod log;
pub mod prelude;
pub mod suspend;
pub mod tasks;
pub mod timers;

pub use editor::*;
pub use flags::*;
pub use plugin::*;

pub use anyhow;
pub use kwark_input::{Chord, Code, Step};

/// Flag for the input tree popup
pub const INPUT_TREE_FLAG: &str = "input_tree_show";
