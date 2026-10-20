use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
    sync::mpsc::{self, Receiver, Sender},
};

use ratatui::{DefaultTerminal, Frame};

use crate::State;
use crate::events::EventStorage;

pub type CallbackFn = Box<dyn FnOnce(&mut Editor) -> anyhow::Result<()> + Send + 'static>;

pub enum Callback {
    Event(TypeId, Box<dyn Any + Send + Sync + 'static>),
    Func(CallbackFn),
}

pub struct Editor {
    pub(crate) term: Option<ratatui::DefaultTerminal>,

    pub state: State,

    pub events: EventStorage,

    pub running: bool,

    callback_rx: Receiver<Callback>,
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
        let (tx, rx) = mpsc::channel();
        Self {
            term: None,

            state: State::new(tx),

            events: EventStorage::default(),

            running: true,

            callback_rx: rx,
        }
    }
}

impl Editor {
    /// Flush and call all events
    pub fn flush(&mut self) -> anyhow::Result<()> {
        loop {
            let Ok(val) = self.callback_rx.try_recv() else {
                break;
            };

            match val {
                Callback::Func(func) => func(self)?,
                Callback::Event(type_, data) => {
                    self.events.handle_any(&mut self.state, type_, data)?
                }
            }
        }

        Ok(())
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
pub struct CallbackSender(pub(crate) Sender<Callback>);

impl CallbackSender {
    pub fn send_fn(
        &mut self,
        func: impl FnOnce(&mut Editor) -> anyhow::Result<()> + Send + Sync + 'static,
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

/// The screen position of the most recent mouse event, read by chords bound to a mouse press
#[derive(Default, Clone, Copy)]
pub struct MousePosition {
    pub column: u16,
    pub row: u16,
}
