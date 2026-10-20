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

/// A type-erased handle onto an `EventHandler<T>`, dispatchable by runtime `TypeId`
trait ErasedEventHandler {
    fn handle_dyn(&self, state: &mut State, event: &mut dyn Any) -> anyhow::Result<()>;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> ErasedEventHandler for EventHandler<T> {
    fn handle_dyn(&self, state: &mut State, event: &mut dyn Any) -> anyhow::Result<()> {
        let event = event
            .downcast_mut::<T>()
            .expect("event was stored under its own TypeId");
        self.handle(state, event)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A general purpose event subscription and execution system
#[derive(Default)]
pub struct EventStorage {
    events: HashMap<TypeId, Box<dyn ErasedEventHandler>>,
}

impl EventStorage {
    /// Register a subscriber for an event type
    pub fn on<T: 'static>(
        &mut self,
        handler: impl Fn(&mut State, &mut T) -> anyhow::Result<()> + 'static,
    ) {
        self.events
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(EventHandler::<T>::default()))
            .as_any_mut()
            .downcast_mut::<EventHandler<T>>()
            .expect("Type was stored under type ID and should be downcastable")
            .subscribers
            .push(Box::new(handler));
    }

    /// Handles a single event
    pub fn handle<T: 'static>(&mut self, state: &mut State, event: &mut T) -> anyhow::Result<()> {
        self.events
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(EventHandler::<T>::default()))
            .as_any_mut()
            .downcast_mut::<EventHandler<T>>()
            .expect("Type was stored under type ID and should be downcastable")
            .handle(state, event)
    }

    /// Handles a single event whose concrete type is only known at runtime
    pub fn handle_any(
        &mut self,
        state: &mut State,
        type_id: TypeId,
        mut event: Box<dyn Any>,
    ) -> anyhow::Result<()> {
        let Some(handler) = self.events.get(&type_id) else {
            return Ok(());
        };

        handler.handle_dyn(state, event.as_mut())
    }
}
