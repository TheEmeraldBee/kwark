use std::{
    any::{Any, TypeId},
    collections::HashMap,
    rc::Rc,
};

use crate::Editor;

mod event_kinds;
pub use event_kinds::*;

pub type EventSubscriber<S, T> = Rc<dyn Fn(&mut Editor<S>, &mut T) -> anyhow::Result<()>>;

struct EventHandler<S: 'static, T> {
    subscribers: Vec<EventSubscriber<S, T>>,
}

/// A type-erased handle onto an `EventHandler`, dispatchable by runtime `TypeId`
trait ErasedEventHandler<S: 'static> {
    fn handle_dyn(&self, editor: &mut Editor<S>, event: &mut dyn Any) -> anyhow::Result<()>;

    fn as_any(&self) -> &dyn Any;
}

impl<S: 'static, T: 'static> ErasedEventHandler<S> for EventHandler<S, T> {
    fn handle_dyn(&self, editor: &mut Editor<S>, event: &mut dyn Any) -> anyhow::Result<()> {
        let event = event
            .downcast_mut::<T>()
            .expect("event was stored under its own TypeId");

        for sub in &self.subscribers {
            sub(editor, event)?;
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A general purpose event subscription and execution system
pub struct EventStorage<S: 'static> {
    events: HashMap<TypeId, Rc<dyn ErasedEventHandler<S>>>,
}

impl<S: 'static> Default for EventStorage<S> {
    fn default() -> Self {
        Self {
            events: HashMap::new(),
        }
    }
}

impl<S: 'static> EventStorage<S> {
    /// Register a subscriber for an event type
    pub fn on<T: 'static>(
        &mut self,
        handler: impl Fn(&mut Editor<S>, &mut T) -> anyhow::Result<()> + 'static,
    ) {
        let mut subscribers = self
            .events
            .get(&TypeId::of::<T>())
            .and_then(|x| x.as_any().downcast_ref::<EventHandler<S, T>>())
            .map(|x| x.subscribers.clone())
            .unwrap_or_default();

        subscribers.push(Rc::new(handler));

        self.events
            .insert(TypeId::of::<T>(), Rc::new(EventHandler { subscribers }));
    }

    fn handler(&self, type_id: TypeId) -> Option<Rc<dyn ErasedEventHandler<S>>> {
        self.events.get(&type_id).cloned()
    }
}

impl<S: 'static> Editor<S> {
    /// Runs the subscribers of an event
    pub fn emit<T: 'static>(&mut self, event: &mut T) -> anyhow::Result<()> {
        match self.events.handler(TypeId::of::<T>()) {
            Some(handler) => handler.handle_dyn(self, event),
            None => Ok(()),
        }
    }

    /// Runs the subscribers of an event whose concrete type is only known at runtime
    pub fn emit_any(&mut self, type_id: TypeId, mut event: Box<dyn Any>) -> anyhow::Result<()> {
        match self.events.handler(type_id) {
            Some(handler) => handler.handle_dyn(self, event.as_mut()),
            None => Ok(()),
        }
    }
}
