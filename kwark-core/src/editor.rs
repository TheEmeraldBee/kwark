use std::any::{Any, TypeId};

use kui::Point;
use kwark_bus::{EventReceiver, EventSender, Message};
use kwark_text::CursorOptions;
use kwark_theme::Theme;
use kwark_tree_sitter::store::LanguageStore;
use kwark_window::{LeafId, WindowCtx, WindowLayout};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::{
    Flags,
    commands::CommandRegistry,
    events::EventStorage,
    frame::{FrameInfo, Redraw},
    log::LogState,
    tasks::Tasks,
    timers::Timers,
};

pub type InputState<S> = kwark_input::InputState<Editor<S>>;
pub type InputTree<S> = kwark_input::InputTree<Editor<S>>;
pub type Pipeline<S> = kwark_render::Pipeline<Editor<S>>;
pub type WindowPipeline<S> = kwark_render::WindowPipeline<Editor<S>, WindowCtx>;
pub type EditorPipeline<S> = kwark_render::EditorPipeline<Editor<S>, ()>;
pub type Overlays<S> = kwark_render::Overlays<Editor<S>>;

pub type CallbackFn<S> = Box<dyn FnOnce(&mut Editor<S>) -> anyhow::Result<()> + Send + 'static>;

pub enum Callback<S: 'static> {
    Event(TypeId, Box<dyn Any + Send + Sync + 'static>),
    Func(CallbackFn<S>),
}

/// A message for the main loop
pub enum Incoming<S: 'static> {
    Callback(Callback<S>),
    Event(Message),
}

/// The editor, with its built-in state and the state of its plugins
pub struct Editor<S: 'static> {
    pub core: Core<S>,
    pub state: S,
    pub events: EventStorage<S>,

    io: Io<S>,
}

/// The state that every editor has
pub struct Core<S: 'static> {
    pub buffers: kwark_buffer::Storage,
    pub windows: WindowLayout,
    pub input: InputState<S>,
    pub commands: CommandRegistry<Editor<S>>,
    pub cursor: CursorOptions,
    pub languages: LanguageStore,
    pub theme: Theme,
    pub flags: Flags,
    pub log: LogState,
    /// The pointer position of the most recent mouse event, read by chords bound to a mouse press
    pub mouse: Point,
    pub running: Running,

    pub pipeline: Pipeline<S>,
    pub window_pipeline: WindowPipeline<S>,
    pub editor_pipeline: EditorPipeline<S>,
    pub overlays: Overlays<S>,

    pub timers: Timers<Editor<S>>,
    pub tasks: Tasks,
    pub redraw: Redraw,
    pub frame: FrameInfo,
}

impl<S: 'static> Core<S> {
    fn new(bus: EventSender) -> Self {
        Self {
            buffers: kwark_buffer::Storage::new(bus.clone()),
            windows: WindowLayout::new(bus.clone()),
            input: InputState::new("normal", bus),
            commands: CommandRegistry::default(),
            cursor: CursorOptions::new(),
            languages: LanguageStore::default(),
            theme: Theme::default(),
            flags: Flags::default(),
            log: LogState::default(),
            mouse: Point::default(),
            running: Running(true),

            pipeline: Pipeline::default(),
            window_pipeline: WindowPipeline::default(),
            editor_pipeline: EditorPipeline::default(),
            overlays: Overlays::default(),

            timers: Timers::default(),
            tasks: Tasks::default(),
            redraw: Redraw::default(),
            frame: FrameInfo::default(),
        }
    }

    pub fn active_buffer(&mut self) -> Option<&mut kwark_buffer::Buffer> {
        let id = self.windows.active().selected()?;
        self.buffers.get_mut(&id)
    }
}

/// The senders that reach the main loop
pub struct Senders<S: 'static> {
    pub callback: CallbackSender<S>,
    pub bus: EventSender,
}

/// The channels and runtime owned by the editor
struct Io<S: 'static> {
    runtime: tokio::runtime::Runtime,
    senders: Senders<S>,
    /// The receivers, until a main loop takes them
    inbox: Option<Inbox<S>>,
}

/// The receiving ends of the callback channel and the event bus
pub struct Inbox<S: 'static> {
    callback_rx: UnboundedReceiver<Callback<S>>,
    bus_rx: EventReceiver,
}

impl<S: 'static> Inbox<S> {
    /// Waits for the next callback or bus event, None once every sender is gone
    pub async fn recv(&mut self) -> Option<Incoming<S>> {
        tokio::select! {
            Some(callback) = self.callback_rx.recv() => Some(Incoming::Callback(callback)),
            Some(event) = self.bus_rx.recv() => Some(Incoming::Event(event)),
            else => None,
        }
    }
}

impl<S: 'static> Editor<S> {
    /// Returns an editor with the state from `state`, after `register` runs on it
    pub fn build(
        state: impl FnOnce() -> anyhow::Result<S>,
        register: impl FnOnce(&mut Self) -> anyhow::Result<()>,
    ) -> anyhow::Result<Self> {
        let (tx, callback_rx) = mpsc::unbounded_channel();
        let (bus, bus_rx) = kwark_bus::channel();

        let senders = Senders {
            callback: CallbackSender(tx),
            bus: bus.clone(),
        };

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let mut editor = Self {
            core: Core::new(bus),
            state: state()?,
            events: EventStorage::default(),

            io: Io {
                runtime,
                senders,
                inbox: Some(Inbox {
                    callback_rx,
                    bus_rx,
                }),
            },
        };

        crate::syntax::register(&mut editor);

        register(&mut editor)?;

        Ok(editor)
    }

    /// Returns a sender for callbacks to the main loop
    pub fn sender(&self) -> CallbackSender<S> {
        self.io.senders.callback.clone()
    }

    /// Returns a sender for the event bus
    pub fn bus(&self) -> EventSender {
        self.io.senders.bus.clone()
    }

    /// Returns the handle of the async runtime
    pub fn runtime(&self) -> &tokio::runtime::Handle {
        self.io.runtime.handle()
    }

    /// Takes the receivers for a main loop to wait on, None if one already did
    pub fn take_inbox(&mut self) -> Option<Inbox<S>> {
        self.io.inbox.take()
    }

    /// Runs a callback or the subscribers for a bus event
    pub fn handle(&mut self, incoming: Incoming<S>) -> anyhow::Result<()> {
        match incoming {
            Incoming::Callback(callback) => self.handle_callback(callback),
            Incoming::Event((type_, data)) => self.emit_any(type_, data),
        }
    }

    /// Returns the next callback that is already queued
    pub fn try_recv_callback(&mut self) -> Option<Callback<S>> {
        self.io.inbox.as_mut()?.callback_rx.try_recv().ok()
    }

    /// Returns the next event on the bus that is already queued
    pub fn try_recv_event(&mut self) -> Option<Message> {
        self.io.inbox.as_mut()?.bus_rx.try_recv()
    }

    /// Runs a single callback
    pub fn handle_callback(&mut self, callback: Callback<S>) -> anyhow::Result<()> {
        match callback {
            Callback::Func(func) => func(self),
            Callback::Event(type_, data) => self.emit_any(type_, data),
        }
    }

    /// Runs `f` with the layout, buffers, and render pipelines lent out of the editor
    pub fn with_view<R>(&mut self, f: impl FnOnce(&mut View<S>, &mut Self) -> R) -> R {
        // Placeholders stay detached from the real bus
        let detached = kwark_bus::channel().0;

        // The stand-in keeps the metrics, so code that runs meanwhile measures the same way
        let stand_in = Pipeline::new(self.core.pipeline.metrics.clone());

        let mut view = View {
            buffers: std::mem::replace(
                &mut self.core.buffers,
                kwark_buffer::Storage::new(detached.clone()),
            ),
            windows: std::mem::replace(&mut self.core.windows, WindowLayout::new(detached)),
            pipeline: std::mem::replace(&mut self.core.pipeline, stand_in),
            window_pipeline: std::mem::take(&mut self.core.window_pipeline),
            editor_pipeline: std::mem::take(&mut self.core.editor_pipeline),
            overlays: std::mem::take(&mut self.core.overlays),
        };

        let out = f(&mut view, self);

        self.core.buffers = view.buffers;
        self.core.windows = view.windows;
        self.core.pipeline = view.pipeline;
        self.core.window_pipeline = view.window_pipeline;
        self.core.editor_pipeline = view.editor_pipeline;
        self.core.overlays = view.overlays;

        out
    }

    /// Returns the window, buffer, and text position under a screen point
    pub fn hit_test(
        &mut self,
        point: Point,
    ) -> Option<(LeafId, kwark_buffer::ID, kwark_buffer::HitPosition)> {
        self.with_view(|v, e| v.windows.hit_test(&v.buffers, &mut v.pipeline, e, point))
    }

    /// Returns the top left on screen of a line and column in a window's selected buffer
    pub fn cell_of(&mut self, leaf: LeafId, pos: (usize, usize)) -> Option<Point> {
        self.with_view(|v, e| v.windows.cell_of(leaf, &v.buffers, &mut v.pipeline, e, pos))
    }
}

/// The layout, buffers, and render pipelines while they are lent out of the editor
pub struct View<S: 'static> {
    pub buffers: kwark_buffer::Storage,
    pub windows: WindowLayout,
    pub pipeline: Pipeline<S>,
    pub window_pipeline: WindowPipeline<S>,
    pub editor_pipeline: EditorPipeline<S>,
    pub overlays: Overlays<S>,
}

/// A type used to send Callbacks through mpsc
pub struct CallbackSender<S: 'static>(UnboundedSender<Callback<S>>);

impl<S: 'static> Clone for CallbackSender<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S: 'static> CallbackSender<S> {
    pub fn send_fn(
        &mut self,
        func: impl FnOnce(&mut Editor<S>) -> anyhow::Result<()> + Send + 'static,
    ) {
        let _ = self.0.send(Callback::Func(Box::new(func)));
    }

    pub fn send_event<T: Any + Send + Sync + 'static>(&mut self, data: T) {
        let _ = self
            .0
            .send(Callback::Event(TypeId::of::<T>(), Box::new(data)));
    }
}

pub struct Running(pub bool);

impl Running {
    pub fn quit(&mut self) {
        self.0 = false
    }
}
