use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

use kwark_bus::{EventReceiver, Message};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use ratatui::{DefaultTerminal, Frame};

use crate::State;
use crate::events::EventStorage;

pub type CallbackFn = Box<dyn FnOnce(&mut Editor) -> anyhow::Result<()> + Send + 'static>;

pub enum Callback {
    Event(TypeId, Box<dyn Any + Send + Sync + 'static>),
    Func(CallbackFn),
}

/// A message for the main loop
pub enum Incoming {
    Callback(Callback),
    Event(Message),
}

pub struct Editor {
    pub(crate) term: Option<ratatui::DefaultTerminal>,

    pub state: State,

    pub events: EventStorage,

    pub running: bool,

    callback_rx: UnboundedReceiver<Callback>,
    bus_rx: EventReceiver,
}

impl Deref for Editor {
    type Target = State;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl DerefMut for Editor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl Default for Editor {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (bus, bus_rx) = kwark_bus::channel();
        Self {
            term: None,

            state: State::new(tx, bus),

            events: EventStorage::default(),

            running: true,

            callback_rx: rx,
            bus_rx,
        }
    }
}

impl Editor {
    /// Waits for the next callback or bus event
    pub async fn recv(&mut self) -> Option<Incoming> {
        tokio::select! {
            Some(callback) = self.callback_rx.recv() => Some(Incoming::Callback(callback)),
            Some(event) = self.bus_rx.recv() => Some(Incoming::Event(event)),
            else => None,
        }
    }

    /// Runs a callback or the subscribers for a bus event
    pub fn handle(&mut self, incoming: Incoming) -> anyhow::Result<()> {
        match incoming {
            Incoming::Callback(callback) => self.handle_callback(callback),
            Incoming::Event(event) => self.handle_event(event),
        }
    }

    /// Returns the next callback that is already queued
    pub fn try_recv_callback(&mut self) -> Option<Callback> {
        self.callback_rx.try_recv().ok()
    }

    /// Returns the next event on the bus that is already queued
    pub fn try_recv_event(&mut self) -> Option<Message> {
        self.bus_rx.try_recv()
    }

    /// Runs the subscribers for an event from the bus
    pub fn handle_event(&mut self, (type_, data): Message) -> anyhow::Result<()> {
        self.events.handle_any(&mut self.state, type_, data)
    }

    /// Runs a single callback
    pub fn handle_callback(&mut self, callback: Callback) -> anyhow::Result<()> {
        match callback {
            Callback::Func(func) => func(self),
            Callback::Event(type_, data) => self.events.handle_any(&mut self.state, type_, data),
        }
    }

    pub(crate) fn take_term(&mut self) -> DefaultTerminal {
        self.term
            .take()
            .expect("terminal should exist when it's accessed")
    }

    pub fn set_term(&mut self, term: DefaultTerminal) {
        self.term = Some(term)
    }

    /// Draws a single frame
    pub fn draw(&mut self, func: impl FnOnce(&mut Frame, &mut State)) -> anyhow::Result<()> {
        let mut term = self.take_term();

        term.draw(|frame| func(frame, &mut self.state))?;

        self.set_term(term);

        Ok(())
    }

    pub fn term(&mut self) -> &mut DefaultTerminal {
        self.term
            .as_mut()
            .expect("terminal should exist when it's accessed")
    }
}

/// A type used to send Callbacks through mpsc
#[derive(Clone)]
pub struct CallbackSender(pub(crate) UnboundedSender<Callback>);

impl CallbackSender {
    pub fn send_fn(
        &mut self,
        func: impl FnOnce(&mut Editor) -> anyhow::Result<()> + Send + 'static,
    ) {
        let _ = self.0.send(Callback::Func(Box::new(func)));
    }

    pub fn send_event<T: Any + Send + Sync + 'static>(&mut self, data: T) {
        let _ = self
            .0
            .send(Callback::Event(TypeId::of::<T>(), Box::new(data)));
    }
}

/// The async runtime for background work
#[derive(Clone)]
pub struct Runtime(pub tokio::runtime::Handle);

impl Deref for Runtime {
    type Target = tokio::runtime::Handle;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Running(pub bool);

impl Running {
    pub fn quit(&mut self) {
        self.0 = false
    }
}

/// The screen position of the most recent mouse event, read by chords bound to a mouse press
#[derive(Default, Clone, Copy)]
pub struct MousePosition {
    pub column: u16,
    pub row: u16,
}
