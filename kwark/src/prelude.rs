// Editor prelude, re-exports the common editor types and crates

pub use crate::Editor;
pub use crate::Flags;
pub use crate::InputState;
pub use crate::InputTree;
pub use crate::MousePosition;
pub use crate::Running;
pub use crate::State;

pub use crate::commands::{ArgValue, Args, CommandRegistry, Group, InputTreeExt};

pub use crate::EditorPipeline;
pub use crate::Pipeline;
pub use crate::WindowPipeline;

pub use crate::layout::{LeafId, Window, WindowCtx, WindowLayout};

pub use crate::replace as replace_term;

pub use crate::pipelines;

pub use crossterm::event::{KeyCode, KeyModifiers};
pub use kwark_input::{Chord, Code, Step};

pub use kwark_buffer as buffer;
pub use kwark_text_buffer as text_buffer;
pub use kwark_text_buffer_renderer as text_render;
pub use kwark_theme::Theme;

pub use kwark_tree_sitter::prelude as tree_sitter;
