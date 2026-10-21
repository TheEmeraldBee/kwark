mod editor;
mod state;

pub mod commands;
pub mod events;
pub mod frame;
pub mod log;
pub mod prelude;
pub mod suspend;
pub mod tasks;
pub mod timers;

pub use editor::*;
pub use state::*;

pub use kwark_input::{Chord, Code, Step};

pub type InputState = kwark_input::InputState<State>;
pub type InputTree = kwark_input::InputTree<State>;
pub type Pipeline = kwark_render::Pipeline<State>;
pub type EditorPipeline = kwark_render::EditorPipeline<State, ()>;

/// An "installable" set of tooling to add to the editor
pub trait Plugin {
    fn register(&self, editor: &mut Editor) -> anyhow::Result<()>;
}

impl Editor {
    /// Installs a plugin
    pub fn install(&mut self, plugin: impl Plugin) -> anyhow::Result<()> {
        plugin.register(self)
    }
}
