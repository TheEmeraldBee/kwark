use std::ops::{BitOr, BitOrAssign};

/// A key by its meaning in the current layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Tab,
    Enter,
    Esc,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    F(u8),
}

/// A mouse button that a chord can bind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// A set of held modifier keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyModifiers(u8);

impl KeyModifiers {
    pub const NONE: Self = Self(0);
    pub const SHIFT: Self = Self(1);
    pub const CONTROL: Self = Self(1 << 1);
    pub const ALT: Self = Self(1 << 2);
    pub const SUPER: Self = Self(1 << 3);
    pub const META: Self = Self(1 << 4);
    pub const HYPER: Self = Self(1 << 5);

    pub const fn empty() -> Self {
        Self::NONE
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every modifier in `other` is held
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }
}

impl BitOr for KeyModifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for KeyModifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn modifiers_combine_and_split() {
        let mut mods = KeyModifiers::CONTROL | KeyModifiers::ALT;

        assert!(mods.contains(KeyModifiers::CONTROL));
        assert!(!mods.contains(KeyModifiers::SHIFT));
        assert!(KeyModifiers::NONE.is_empty());

        mods.remove(KeyModifiers::CONTROL);
        assert_eq!(mods, KeyModifiers::ALT);
    }
}
