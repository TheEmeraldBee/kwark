use std::any::{Any, TypeId};

use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// A queued event and the type it was sent as
pub type Message = (TypeId, Box<dyn Any + Send + Sync>);

/// Creates a connected event sender and receiver
pub fn channel() -> (EventSender, EventReceiver) {
    let (tx, rx) = mpsc::unbounded_channel();
    (EventSender(tx), EventReceiver(rx))
}

/// The sending half of the event bus
#[derive(Clone)]
pub struct EventSender(UnboundedSender<Message>);

impl EventSender {
    /// Queues an event for the main loop
    pub fn emit<T: Any + Send + Sync>(&self, event: T) {
        let _ = self.0.send((TypeId::of::<T>(), Box::new(event)));
    }
}

/// The receiving half of the event bus
pub struct EventReceiver(UnboundedReceiver<Message>);

impl EventReceiver {
    /// Waits for the next event
    pub async fn recv(&mut self) -> Option<Message> {
        self.0.recv().await
    }

    /// Returns the next event that is already queued
    pub fn try_recv(&mut self) -> Option<Message> {
        self.0.try_recv().ok()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn emits_in_order_with_type() {
        let (tx, mut rx) = channel();

        tx.emit(1u32);
        tx.emit("two");

        let (id, value) = rx.try_recv().unwrap();
        assert_eq!(id, TypeId::of::<u32>());
        assert_eq!(value.downcast_ref::<u32>(), Some(&1));

        let (id, _) = rx.try_recv().unwrap();
        assert_eq!(id, TypeId::of::<&str>());
        assert!(rx.try_recv().is_none());
    }
}
