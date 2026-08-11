use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    ops::{Deref, DerefMut},
    sync::mpsc::{self, Receiver, Sender},
};

use crate::events::EventStorage;

pub type CallbackFn = Box<dyn FnOnce(&mut Editor) -> anyhow::Result<()> + Send + 'static>;

pub enum Callback {
    Event(TypeId, Box<dyn Any>),
    Func(CallbackFn),
}

pub struct Editor {
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
}

/// A type used to send Callbacks through mpsc
pub struct CallbackSender(Sender<Callback>);

impl CallbackSender {
    pub fn send_fn(
        &mut self,
        func: impl FnOnce(&mut Editor) -> anyhow::Result<()> + Send + 'static,
    ) {
        let _ = self.0.send(Callback::Func(Box::new(func)));
    }

    pub fn send_event<T: Any + 'static>(&mut self, data: T) {
        let _ = self
            .0
            .send(Callback::Event(TypeId::of::<T>(), Box::new(data)));
    }
}

pub struct State {
    callback_tx: Sender<Callback>,
    inner: HashMap<TypeId, Box<dyn Any>>,
}

impl State {
    pub fn new(callback_tx: Sender<Callback>) -> Self {
        Self {
            callback_tx,
            inner: HashMap::default(),
        }
    }

    /// Insert a type into the typemap
    pub fn insert<T: Any + 'static>(&mut self, data: T) {
        self.inner.insert(TypeId::of::<T>(), Box::new(data));
    }

    /// Gets a set of items out from the TypeMap
    pub fn get<'a, T: GetManyAny<'a>>(&'a mut self) -> T {
        T::get_many_any(&mut self.inner).expect("Types should be checked and inserted early")
    }

    /// Returns the sender for a callback fn to be put into a thread/event
    pub fn sender(&self) -> CallbackSender {
        CallbackSender(self.callback_tx.clone())
    }
}

#[derive(Debug)]
pub enum StateError {
    NotFound(&'static str),
    ConflictingAccess(&'static str),
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateError::NotFound(name) => write!(f, "type `{name}` not registered in state"),
            StateError::ConflictingAccess(name) => {
                write!(
                    f,
                    "type `{name}` requested more than once in the same access"
                )
            }
        }
    }
}

impl std::error::Error for StateError {}

trait StateGet<'a>: Sized {
    fn type_id() -> TypeId;
    fn get(slot: &'a mut Box<dyn Any>) -> Self;
}

impl<'a, T: Any + 'static> StateGet<'a> for &'a T {
    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }

    fn get(slot: &'a mut Box<dyn Any>) -> Self {
        slot.downcast_ref::<T>()
            .expect("slot was fetched by this exact TypeId")
    }
}

impl<'a, T: Any + 'static> StateGet<'a> for &'a mut T {
    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }

    fn get(slot: &'a mut Box<dyn Any>) -> Self {
        slot.downcast_mut::<T>()
            .expect("slot was fetched by this exact TypeId")
    }
}

pub trait GetManyAny<'a>: Sized {
    fn get_many_any(inner: &'a mut HashMap<TypeId, Box<dyn Any>>) -> Result<Self, StateError>;
}

impl<'a, T: StateGet<'a>> GetManyAny<'a> for T {
    fn get_many_any(inner: &'a mut HashMap<TypeId, Box<dyn Any>>) -> Result<Self, StateError> {
        let id = T::type_id();
        inner
            .get_mut(&id)
            .map(T::get)
            .ok_or(StateError::NotFound(std::any::type_name::<T>()))
    }
}

macro_rules! impl_get_many_any_tuple {
    ($($T:ident : $idx:tt),+) => {
        impl<'a, $($T: StateGet<'a>),+> GetManyAny<'a> for ($($T,)+) {
            #[allow(non_snake_case)]
            fn get_many_any(
                inner: &'a mut HashMap<TypeId, Box<dyn Any>>,
            ) -> Result<Self, StateError> {
                let ids = [$(($T::type_id(), std::any::type_name::<$T>())),+];
                for i in 0..ids.len() {
                    for j in (i + 1)..ids.len() {
                        if ids[i].0 == ids[j].0 {
                            return Err(StateError::ConflictingAccess(ids[i].1));
                        }
                    }
                }
                let [$($T,)+] = inner.get_disjoint_mut([$(&ids[$idx].0),+]);
                Ok(($(
                    $T.map(<$T as StateGet<'a>>::get)
                        .ok_or(StateError::NotFound(std::any::type_name::<$T>()))?,
                )+))
            }
        }
    };
}

impl_get_many_any_tuple!(A:0, B:1);
impl_get_many_any_tuple!(A:0, B:1, C:2);
impl_get_many_any_tuple!(A:0, B:1, C:2, D:3);
impl_get_many_any_tuple!(A:0, B:1, C:2, D:3, E:4);
impl_get_many_any_tuple!(A:0, B:1, C:2, D:3, E:4, F:5);
impl_get_many_any_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6);
impl_get_many_any_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7);

#[cfg(test)]
mod tests {
    use super::*;

    fn new_state() -> State {
        let tx = mpsc::channel().0;
        State::new(tx)
    }

    #[test]
    fn gets_disjoint_pairs() {
        let mut state = new_state();

        state.insert(1u32);
        state.insert("hi".to_string());
        state.insert(3.5f64);

        let (a, b, c) = state.get::<(&mut u32, &String, &mut f64)>();
        *a += 1;
        assert_eq!(*a, 2);
        assert_eq!(b, "hi");
        *c += 0.5;
        assert_eq!(*c, 4.0);
    }
}
