use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    ops::{Deref, DerefMut},
    sync::mpsc::{self, Receiver, Sender},
};

use ratatui::DefaultTerminal;

use crate::events::EventStorage;

pub type CallbackFn = Box<dyn FnOnce(&mut Editor) -> anyhow::Result<()> + Send + 'static>;

pub enum Callback {
    Event(TypeId, Box<dyn Any>),
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

    pub(crate) fn set_term(&mut self, term: DefaultTerminal) {
        self.term = Some(term)
    }

    pub fn term(&mut self) -> &mut DefaultTerminal {
        self.term
            .as_mut()
            .expect("terminal should exist when it's accessed")
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

/// A set of flags (booleans) that the editor can quickly hold
#[derive(Default)]
pub struct Flags {
    flags: HashMap<&'static str, bool>,
}

impl Flags {
    /// Get the value of a flag
    pub fn get(&self, flag: &str) -> bool {
        self.flags.get(flag).copied().unwrap_or_default()
    }

    /// Set the value of a flag
    pub fn set(&mut self, flag: &'static str, set: bool) {
        self.flags.insert(flag, set);
    }

    /// Toggle the value of a flag
    pub fn toggle(&mut self, flag: &'static str) {
        self.flags.insert(flag, !self.get(flag));
    }
}

pub struct State {
    callback_tx: Sender<Callback>,
    inner: HashMap<TypeId, Box<dyn Any>>,
}

impl State {
    pub fn new(callback_tx: Sender<Callback>) -> Self {
        let mut val = Self {
            callback_tx,
            inner: HashMap::default(),
        };

        val.insert(Flags::default());

        val
    }

    /// Insert a type into the typemap
    pub fn insert<T: Any + 'static>(&mut self, data: T) {
        self.inner.insert(TypeId::of::<T>(), Box::new(data));
    }

    /// Gets a set of items out from the TypeMap
    pub fn get<'a, T: GetManyAny<'a>>(&'a mut self) -> T {
        T::get_many_any(&mut self.inner).expect("Types should be checked and inserted early")
    }

    /// Pull/take the value out of the editor
    ///
    /// # IMPORTANT
    /// You should almost **always** insert the
    /// state back into the editor when you are done with it
    pub fn take<T: Any + 'static>(&mut self) -> T {
        *self
            .inner
            .remove(&TypeId::of::<T>())
            .expect("Type expected to exist within the editor")
            .downcast::<T>()
            .expect("Value should be the right type")
    }

    /// Returns the sender for a callback fn to be put into a thread/event
    pub fn sender(&self) -> CallbackSender {
        CallbackSender(self.callback_tx.clone())
    }

    /// Removes one or more types from the state, returning a guard that puts
    /// them back when dropped
    pub fn scope<T: PutBack>(&mut self) -> Taken<'_, T> {
        let value = T::take_all(self);
        Taken {
            state: self,
            value: Some(value),
        }
    }
}

/// A set of types temporarily removed from [`State`] by [`State::scope`]
pub struct Taken<'a, T: PutBack> {
    state: &'a mut State,
    value: Option<T>,
}

impl<'a, T: PutBack> Taken<'a, T> {
    /// The removed value and the rest of the state
    pub fn split(&mut self) -> (&mut T, &mut State) {
        (
            self.value.as_mut().expect("value present until dropped"),
            self.state,
        )
    }
}

impl<'a, T: PutBack> Deref for Taken<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value.as_ref().expect("value present until dropped")
    }
}

impl<'a, T: PutBack> DerefMut for Taken<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.value.as_mut().expect("value present until dropped")
    }
}

impl<'a, T: PutBack> Drop for Taken<'a, T> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            value.put_back(self.state);
        }
    }
}

/// A tuple of types that can be removed from [`State`] as a group, and put back as a group
pub trait PutBack: Sized {
    fn take_all(state: &mut State) -> Self;
    fn put_back(self, state: &mut State);
}

macro_rules! impl_put_back_tuple {
    ($($T:ident),+) => {
        impl<$($T: Any + 'static),+> PutBack for ($($T,)+) {
            fn take_all(state: &mut State) -> Self {
                ($(state.take::<$T>(),)+)
            }

            #[allow(non_snake_case)]
            fn put_back(self, state: &mut State) {
                let ($($T,)+) = self;
                $(state.insert($T);)+
            }
        }
    };
}

impl_put_back_tuple!(A);
impl_put_back_tuple!(A, B);
impl_put_back_tuple!(A, B, C);
impl_put_back_tuple!(A, B, C, D);

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
