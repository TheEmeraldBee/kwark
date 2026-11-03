use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    fmt,
};

/// A set of values stored by type
#[derive(Default)]
pub struct Resources {
    inner: HashMap<TypeId, Entry>,
}

struct Entry {
    name: &'static str,
    value: Box<dyn Any>,
}

impl Resources {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a value, returning the previous value of the same type
    pub fn insert<T: Any>(&mut self, value: T) -> Option<T> {
        let entry = Entry {
            name: type_name::<T>(),
            value: Box::new(value),
        };

        self.inner
            .insert(TypeId::of::<T>(), entry)
            .and_then(|x| x.value.downcast().ok().map(|x| *x))
    }

    /// Removes and returns the value of type `T`
    pub fn remove<T: Any>(&mut self) -> Option<T> {
        let entry = self.inner.remove(&TypeId::of::<T>())?;
        entry.value.downcast().ok().map(|x| *x)
    }

    pub fn contains<T: Any>(&self) -> bool {
        self.inner.contains_key(&TypeId::of::<T>())
    }

    /// Returns shared references, as `&T` or a tuple of them
    pub fn get_ref<'a, T: Fetch<'a>>(&'a self) -> Result<T, Error> {
        T::fetch(self)
    }

    /// Returns shared or mutable references, as `&T`, `&mut T`, or a tuple of them
    pub fn get<'a, T: FetchMut<'a>>(&'a mut self) -> Result<T, Error> {
        T::fetch_mut(self)
    }

    /// Returns mutable references, with each missing value inserted as default
    pub fn get_or_default<'a, T: FetchDefault<'a>>(&'a mut self) -> T {
        T::insert_defaults(self);
        T::fetch_mut(self).unwrap_or_else(|e| panic!("{e}"))
    }

    /// Returns the type names of the stored values
    pub fn names(&self) -> impl Iterator<Item = &'static str> {
        self.inner.values().map(|x| x.name)
    }

    fn insert_default<T: Any + Default>(&mut self) {
        if !self.contains::<T>() {
            self.insert(T::default());
        }
    }
}

/// An error from a fetch on [`Resources`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The type is not stored
    NotFound(&'static str),
    /// The type appears more than once in a mutable fetch
    Conflict(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound(name) => write!(f, "type `{name}` is not stored"),
            Error::Conflict(name) => {
                write!(f, "type `{name}` is requested more than once in a mutable fetch")
            }
        }
    }
}

impl std::error::Error for Error {}

/// A reference that a shared fetch can return
pub trait Ref<'a>: Sized {
    type Target: Any;

    fn from_any(value: &'a dyn Any) -> Self;
}

impl<'a, T: Any> Ref<'a> for &'a T {
    type Target = T;

    fn from_any(value: &'a dyn Any) -> Self {
        value.downcast_ref().expect("value matches its type id")
    }
}

/// A reference that a mutable fetch can return
pub trait Mut<'a>: Sized {
    type Target: Any;

    fn from_any(value: &'a mut dyn Any) -> Self;
}

impl<'a, T: Any> Mut<'a> for &'a T {
    type Target = T;

    fn from_any(value: &'a mut dyn Any) -> Self {
        <&T as Ref>::from_any(value)
    }
}

impl<'a, T: Any> Mut<'a> for &'a mut T {
    type Target = T;

    fn from_any(value: &'a mut dyn Any) -> Self {
        value.downcast_mut().expect("value matches its type id")
    }
}

/// One or more shared references taken from [`Resources`]
pub trait Fetch<'a>: Sized {
    fn fetch(res: &'a Resources) -> Result<Self, Error>;
}

/// One or more shared or mutable references taken from [`Resources`]
pub trait FetchMut<'a>: Sized {
    fn fetch_mut(res: &'a mut Resources) -> Result<Self, Error>;
}

/// A [`FetchMut`] whose types have a default value
pub trait FetchDefault<'a>: FetchMut<'a> {
    fn insert_defaults(res: &mut Resources);
}

fn find<'a, R: Ref<'a>>(res: &'a Resources) -> Result<R, Error> {
    res.inner
        .get(&TypeId::of::<R::Target>())
        .map(|x| R::from_any(x.value.as_ref()))
        .ok_or(Error::NotFound(type_name::<R::Target>()))
}

impl<'a, R: Ref<'a>> Fetch<'a> for R {
    fn fetch(res: &'a Resources) -> Result<Self, Error> {
        find(res)
    }
}

impl<'a, M: Mut<'a>> FetchMut<'a> for M {
    fn fetch_mut(res: &'a mut Resources) -> Result<Self, Error> {
        res.inner
            .get_mut(&TypeId::of::<M::Target>())
            .map(|x| M::from_any(x.value.as_mut()))
            .ok_or(Error::NotFound(type_name::<M::Target>()))
    }
}

impl<'a, M: Mut<'a>> FetchDefault<'a> for M
where
    M::Target: Default,
{
    fn insert_defaults(res: &mut Resources) {
        res.insert_default::<M::Target>();
    }
}

macro_rules! impl_fetch_tuple {
    ($($T:ident : $idx:tt),+) => {
        impl<'a, $($T: Ref<'a>),+> Fetch<'a> for ($($T,)+) {
            fn fetch(res: &'a Resources) -> Result<Self, Error> {
                Ok(($(find::<$T>(res)?,)+))
            }
        }

        impl<'a, $($T: Mut<'a>),+> FetchMut<'a> for ($($T,)+) {
            #[allow(non_snake_case)]
            fn fetch_mut(res: &'a mut Resources) -> Result<Self, Error> {
                let ids = [$((TypeId::of::<$T::Target>(), type_name::<$T::Target>())),+];

                for i in 0..ids.len() {
                    if ids[i + 1..].iter().any(|x| x.0 == ids[i].0) {
                        return Err(Error::Conflict(ids[i].1));
                    }
                }

                let [$($T,)+] = res.inner.get_disjoint_mut([$(&ids[$idx].0),+]);

                Ok(($(
                    $T.map(|x| <$T as Mut<'a>>::from_any(x.value.as_mut()))
                        .ok_or(Error::NotFound(ids[$idx].1))?,
                )+))
            }
        }

        impl<'a, $($T: Mut<'a>),+> FetchDefault<'a> for ($($T,)+)
        where
            $($T::Target: Default,)+
        {
            fn insert_defaults(res: &mut Resources) {
                $(res.insert_default::<$T::Target>();)+
            }
        }
    };
}

impl_fetch_tuple!(A:0, B:1);
impl_fetch_tuple!(A:0, B:1, C:2);
impl_fetch_tuple!(A:0, B:1, C:2, D:3);
impl_fetch_tuple!(A:0, B:1, C:2, D:3, E:4);
impl_fetch_tuple!(A:0, B:1, C:2, D:3, E:4, F:5);
impl_fetch_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6);
impl_fetch_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7);

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Default, Debug, PartialEq)]
    struct Scroll(usize);

    #[derive(Default, Debug, PartialEq)]
    struct Name(&'static str);

    #[test]
    pub fn insert_returns_the_previous_value() {
        let mut res = Resources::new();

        assert_eq!(res.insert(Scroll(1)), None);
        assert_eq!(res.insert(Scroll(2)), Some(Scroll(1)));
        assert_eq!(res.remove::<Scroll>(), Some(Scroll(2)));
        assert!(!res.contains::<Scroll>());
    }

    #[test]
    pub fn shared_fetch_allows_repeated_types() {
        let mut res = Resources::new();
        res.insert(Scroll(3));

        let (a, b) = res.get_ref::<(&Scroll, &Scroll)>().unwrap();

        assert_eq!(a, b);
    }

    #[test]
    pub fn mutable_fetch_mixes_shared_and_mutable() {
        let mut res = Resources::new();
        res.insert(Scroll(0));
        res.insert(Name("a"));

        let (scroll, name) = res.get::<(&mut Scroll, &Name)>().unwrap();
        scroll.0 = name.0.len();

        assert_eq!(res.get_ref::<&Scroll>(), Ok(&Scroll(1)));
    }

    #[test]
    pub fn mutable_fetch_rejects_repeated_types() {
        let mut res = Resources::new();
        res.insert(Scroll(0));

        let err = res.get::<(&mut Scroll, &Scroll)>().err();

        assert_eq!(err, Some(Error::Conflict(type_name::<Scroll>())));
    }

    #[test]
    pub fn missing_types_report_their_name() {
        let mut res = Resources::new();

        assert_eq!(
            res.get::<&mut Scroll>().err(),
            Some(Error::NotFound(type_name::<Scroll>()))
        );
    }

    #[test]
    pub fn get_or_default_inserts_missing_values() {
        let mut res = Resources::new();
        res.insert(Name("kept"));

        let (scroll, name) = res.get_or_default::<(&mut Scroll, &mut Name)>();
        scroll.0 = 4;

        assert_eq!(name, &Name("kept"));
        assert_eq!(res.get_ref::<&Scroll>(), Ok(&Scroll(4)));
    }

    #[test]
    pub fn names_lists_stored_types() {
        let mut res = Resources::new();
        res.insert(Scroll(0));

        assert_eq!(res.names().collect::<Vec<_>>(), [type_name::<Scroll>()]);
    }
}
