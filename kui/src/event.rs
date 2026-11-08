use crate::{Point, Size};

/// An input or window event, with positions in logical pixels
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A key press or repeat
    Key(KeyEvent),
    /// Text that an input method committed
    Text(String),
    /// Text that an input method is still composing, with the selected byte range
    Preedit {
        text: String,
        cursor: Option<(usize, usize)>,
    },
    MouseDown {
        position: Point,
        button: MouseButton,
    },
    MouseUp {
        position: Point,
        button: MouseButton,
    },
    MouseMove {
        position: Point,
    },
    /// The pointer left the window
    MouseLeave,
    Scroll {
        position: Point,
        delta: ScrollDelta,
    },
    /// The window has a new inner size
    Resize(Size),
    /// The window has a new number of physical pixels per logical pixel
    Scale(f32),
    /// The window gained or lost focus
    Focus(bool),
    /// The user asked to close the window, which closes unless the app calls `Cx::keep_open`
    CloseRequested,
}

/// A key press and the text it produced
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    /// The text the press typed, absent for control chords and keys that type nothing
    pub text: Option<String>,
    pub mods: Mods,
    /// Whether the press comes from holding the key down
    pub repeat: bool,
}

/// A key by its meaning in the current layout
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// A key that types a character, space included
    Char(char),
    Named(NamedKey),
    /// A key with no name that kui knows
    Unknown,
}

/// A key that types no character
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Enter,
    Tab,
    Backspace,
    Delete,
    Insert,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    /// A function key by its number
    F(u8),
}

/// The modifier keys held during an event
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    /// The Super, Windows, or Command key
    pub logo: bool,
}

/// A mouse button
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

/// How far a scroll moved
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollDelta {
    /// Lines and columns, from a mouse wheel
    Lines(Point),
    /// Logical pixels, from a touchpad
    Pixels(Point),
}

impl From<winit::event::MouseButton> for MouseButton {
    fn from(button: winit::event::MouseButton) -> Self {
        use winit::event::MouseButton as B;

        match button {
            B::Left => Self::Left,
            B::Right => Self::Right,
            B::Middle => Self::Middle,
            B::Back => Self::Back,
            B::Forward => Self::Forward,
            B::Other(n) => Self::Other(n),
        }
    }
}

impl From<winit::keyboard::ModifiersState> for Mods {
    fn from(state: winit::keyboard::ModifiersState) -> Self {
        Self {
            shift: state.shift_key(),
            ctrl: state.control_key(),
            alt: state.alt_key(),
            logo: state.super_key(),
        }
    }
}

impl Key {
    /// Returns the key for a winit key, or `None` for a modifier key on its own
    pub(crate) fn from_winit(key: &winit::keyboard::Key) -> Option<Self> {
        use winit::keyboard::{Key as K, NamedKey as N};

        let named = match key {
            K::Character(s) => {
                let mut chars = s.chars();
                return Some(match (chars.next(), chars.next()) {
                    (Some(c), None) => Self::Char(c),
                    _ => Self::Unknown,
                });
            }
            K::Named(named) => named,
            K::Unidentified(_) | K::Dead(_) => return Some(Self::Unknown),
        };

        let named = match named {
            N::Space => return Some(Self::Char(' ')),
            N::Shift | N::Control | N::Alt | N::AltGraph | N::Super | N::Meta | N::Hyper => {
                return None;
            }
            N::Enter => NamedKey::Enter,
            N::Tab => NamedKey::Tab,
            N::Backspace => NamedKey::Backspace,
            N::Delete => NamedKey::Delete,
            N::Insert => NamedKey::Insert,
            N::Escape => NamedKey::Escape,
            N::ArrowUp => NamedKey::Up,
            N::ArrowDown => NamedKey::Down,
            N::ArrowLeft => NamedKey::Left,
            N::ArrowRight => NamedKey::Right,
            N::Home => NamedKey::Home,
            N::End => NamedKey::End,
            N::PageUp => NamedKey::PageUp,
            N::PageDown => NamedKey::PageDown,
            N::F1 => NamedKey::F(1),
            N::F2 => NamedKey::F(2),
            N::F3 => NamedKey::F(3),
            N::F4 => NamedKey::F(4),
            N::F5 => NamedKey::F(5),
            N::F6 => NamedKey::F(6),
            N::F7 => NamedKey::F(7),
            N::F8 => NamedKey::F(8),
            N::F9 => NamedKey::F(9),
            N::F10 => NamedKey::F(10),
            N::F11 => NamedKey::F(11),
            N::F12 => NamedKey::F(12),
            _ => return Some(Self::Unknown),
        };

        Some(Self::Named(named))
    }
}

#[cfg(test)]
mod test {
    use winit::keyboard::{Key as K, NamedKey as N, SmolStr};

    use super::*;

    #[test]
    fn keys_map_to_chars_names_or_nothing() {
        assert_eq!(
            Key::from_winit(&K::Character(SmolStr::new("a"))),
            Some(Key::Char('a'))
        );
        assert_eq!(Key::from_winit(&K::Named(N::Space)), Some(Key::Char(' ')));
        assert_eq!(
            Key::from_winit(&K::Named(N::F5)),
            Some(Key::Named(NamedKey::F(5)))
        );
        assert_eq!(Key::from_winit(&K::Named(N::Shift)), None);
    }
}
