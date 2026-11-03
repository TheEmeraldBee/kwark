use std::{
    any::Any,
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use kwark_bus::EventSender;
use kwark_resources::{FetchMut, Resources};
use tokio::sync::mpsc::UnboundedSender;

use crate::{Callback, CallbackSender};

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
    callback_tx: UnboundedSender<Callback>,
    bus: EventSender,
    resources: Resources,
}

impl State {
    pub fn new(callback_tx: UnboundedSender<Callback>, bus: EventSender) -> Self {
        let mut val = Self {
            callback_tx,
            bus,
            resources: Resources::new(),
        };

        val.insert(Flags::default());

        val
    }

    /// Insert a type into the typemap
    pub fn insert<T: Any + 'static>(&mut self, data: T) {
        self.resources.insert(data);
    }

    /// Checks whether a type is within the state
    pub fn has<T: Any + 'static>(&self) -> bool {
        self.resources.contains::<T>()
    }

    /// Gets a set of items out from the TypeMap
    pub fn get<'a, T: FetchMut<'a>>(&'a mut self) -> T {
        self.resources.get().unwrap_or_else(|e| panic!("{e}"))
    }

    /// Pull/take the value out of the editor
    ///
    /// # IMPORTANT
    /// You should almost **always** insert the
    /// state back into the editor when you are done with it
    pub fn take<T: Any + 'static>(&mut self) -> T {
        self.resources
            .remove()
            .unwrap_or_else(|| panic!("type `{}` is not stored", std::any::type_name::<T>()))
    }

    /// Returns the sender for a callback fn to be put into a thread/event
    pub fn sender(&self) -> CallbackSender {
        CallbackSender(self.callback_tx.clone())
    }

    /// Returns a sender for the event bus
    pub fn bus(&self) -> EventSender {
        self.bus.clone()
    }

    /// Removes one or more types from the state, returning a guard that puts them back
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
