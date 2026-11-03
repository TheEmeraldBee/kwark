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
/// kwark_core::state! {
///     struct Empty {}
/// }
///
/// fn count(e: &mut Editor<Empty>) -> usize {
///     *e.plugin::<Counter>()
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "the plugin `{P}` is not in `{Self}`",
    label = "this editor state lacks the plugin `{P}`",
    note = "add `{P}` to the `kwark::state!` block that defines `{Self}`"
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

/// Defines the state of an editor from a list of plugins
///
/// ```
/// kwark_core::state! {
///     pub struct MinimalState {}
/// }
///
/// let editor = MinimalState::editor().unwrap();
/// assert!(editor.core.running.0);
/// ```
#[macro_export]
macro_rules! state {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $($field:ident : $plugin:ty $(= $init:expr)?),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $name {
            $(pub $field: <$plugin as $crate::PluginState>::State,)*
        }

        $(
            impl $crate::Has<$plugin> for $name {
                fn plugin(&self) -> &<$plugin as $crate::PluginState>::State {
                    &self.$field
                }

                fn plugin_mut(&mut self) -> &mut <$plugin as $crate::PluginState>::State {
                    &mut self.$field
                }
            }
        )*

        impl $name {
            /// Returns an editor with every plugin of this state registered
            #[allow(unused_variables)]
            pub fn editor() -> $crate::anyhow::Result<$crate::Editor<Self>> {
                $(let $field: $plugin = $crate::__plugin!($plugin $(= $init)?);)*

                $crate::Editor::build(
                    |senders| {
                        Ok(Self {
                            $($field: $crate::PluginState::init(&$field, senders)?,)*
                        })
                    },
                    |editor| {
                        $($crate::Plugin::<Self>::register(&$field, editor)?;)*
                        Ok(())
                    },
                )
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __plugin {
    ($plugin:ty) => {
        <$plugin as ::core::default::Default>::default()
    };
    ($plugin:ty = $init:expr) => {
        $init
    };
}
