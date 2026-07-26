use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::state::State;

mod event_kinds;
pub use event_kinds::*;

pub type EventSubscriber<T> = Box<dyn Fn(&mut State, &mut T) -> anyhow::Result<()>>;

struct EventHandler<T> {
    subscribers: Vec<EventSubscriber<T>>,
}

impl<T> Default for EventHandler<T> {
    fn default() -> Self {
        Self {
            subscribers: vec![],
        }
    }
}

impl<T> EventHandler<T> {
    fn handle(&self, state: &mut State, event: &mut T) -> anyhow::Result<()> {
        for sub in &self.subscribers {
            sub(state, event)?;
        }

        Ok(())
    }
}

/// A general purpose event subscription and execution system
#[derive(Default)]
pub struct EventStorage {
    events: HashMap<TypeId, Box<dyn Any>>,
}

impl EventStorage {
    /// Register a subscriber for an event type
    pub fn on<T: 'static>(
        &mut self,
        handler: impl Fn(&mut State, &mut T) -> anyhow::Result<()> + 'static,
    ) {
        self.events
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(EventHandler::<T>::default()))
            .downcast_mut::<EventHandler<T>>()
            .expect("Type was stored under type ID and should be downcastable")
            .subscribers
            .push(Box::new(handler));
    }

    /// Handles a single event
    pub fn handle<T: 'static>(&mut self, state: &mut State, event: &mut T) -> anyhow::Result<()> {
        self.events
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(EventHandler::<T>::default()))
            .downcast_mut::<EventHandler<T>>()
            .expect("Type was stored under type ID and should be downcastable")
            .handle(state, event)
    }
}
