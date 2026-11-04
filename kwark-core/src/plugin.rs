use crate::{Core, Editor, Senders};

/// A plugin and the state it adds to the editor
pub trait PluginState: Sized + 'static {
    type State: 'static;

    /// Returns the initial state of the plugin
    fn init<S: 'static>(&self, senders: &Senders<S>) -> anyhow::Result<Self::State>;
}

/// A plugin that registers itself into an editor with state `S`
pub trait Plugin<S: 'static>: PluginState {
    /// Registers the commands, bindings, and events of the plugin
    fn register(&self, editor: &mut Editor<S>) -> anyhow::Result<()>;
}

/// Proof that the editor state includes the plugin `P`
///
/// ```compile_fail
/// use kwark_core::{Editor, PluginState, Senders};
///
/// #[derive(Default)]
/// struct Counter;
///
/// impl PluginState for Counter {
///     type State = usize;
///
///     fn init<S: 'static>(&self, _: &Senders<S>) -> anyhow::Result<usize> {
///         Ok(0)
///     }
/// }
///
/// struct Empty;
///
/// fn count(e: &mut Editor<Empty>) -> usize {
///     *e.plugin::<Counter>()
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "the plugin `{P}` is not in `{Self}`",
    label = "this editor state lacks the plugin `{P}`",
    note = "add a `#[plugin]` field of type `{P}` to `{Self}`"
)]
pub trait Has<P: PluginState> {
    fn plugin(&self) -> &P::State;
    fn plugin_mut(&mut self) -> &mut P::State;
}

impl<S: 'static> Editor<S> {
    /// Returns the state of the plugin `P`
    pub fn plugin<P: PluginState>(&self) -> &P::State
    where
        S: Has<P>,
    {
        self.state.plugin()
    }

    /// Returns the state of the plugin `P` mutably
    pub fn plugin_mut<P: PluginState>(&mut self) -> &mut P::State
    where
        S: Has<P>,
    {
        self.state.plugin_mut()
    }

    /// Returns the core and the state of the plugin `P` together
    pub fn split<P: PluginState>(&mut self) -> (&mut Core<S>, &mut P::State)
    where
        S: Has<P>,
    {
        (&mut self.core, self.state.plugin_mut())
    }
}

/// An editor state built from plugins and fields
pub trait State: Sized + 'static {
    /// Returns an editor with every plugin of this state registered
    fn editor() -> anyhow::Result<Editor<Self>>;
}

/// Returns an editor with every plugin of the state `S` registered
pub fn init<S: State>() -> anyhow::Result<Editor<S>> {
    S::editor()
}
