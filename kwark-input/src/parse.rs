use crossterm::event::{KeyCode, KeyModifiers};

use crate::Chord;

/// Error returned when a chord string can't be parsed
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("empty chord string")]
    Empty,
    #[error("unknown modifier {0:?}")]
    UnknownModifier(String),
    #[error("unknown key {0:?}")]
    UnknownKey(String),
}

/// Parses a hyphen-separated chord string (e.g. `"ctrl-alt-d"`) into a `Chord`
pub fn parse_chord(input: &str) -> Result<Chord, ParseError> {
    if input.is_empty() {
        return Err(ParseError::Empty);
    }

    let (mods_str, key_str) = if input == "-" {
        ("", "-")
    } else if let Some(stripped) = input.strip_suffix("--") {
        (stripped, "-")
    } else {
        input.rsplit_once('-').unwrap_or(("", input))
    };

    let mut mods = KeyModifiers::NONE;
    for token in mods_str.split('-').filter(|t| !t.is_empty()) {
        mods |= parse_modifier(token)?;
    }

    let code = parse_key(key_str)?;

    Ok(Chord::new(code, mods))
}

fn parse_modifier(token: &str) -> Result<KeyModifiers, ParseError> {
    match token {
        "ctrl" | "control" => Ok(KeyModifiers::CONTROL),
        "alt" | "opt" | "option" => Ok(KeyModifiers::ALT),
        "shift" => Ok(KeyModifiers::SHIFT),
        "super" | "cmd" | "win" => Ok(KeyModifiers::SUPER),
        "meta" => Ok(KeyModifiers::META),
        "hyper" => Ok(KeyModifiers::HYPER),
        other => Err(ParseError::UnknownModifier(other.to_string())),
    }
}

fn parse_key(token: &str) -> Result<KeyCode, ParseError> {
    let code = match token {
        "-" => KeyCode::Char('-'),
        "space" => KeyCode::Char(' '),
        "tab" => KeyCode::Tab,
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        _ => {
            let mut chars = token.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => KeyCode::Char(c),
                _ => match token.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
                    Some(n) => KeyCode::F(n),
                    None => return Err(ParseError::UnknownKey(token.to_string())),
                },
            }
        }
    };

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_key() {
        assert_eq!(
            parse_chord("d").unwrap(),
            Chord::new(KeyCode::Char('d'), KeyModifiers::NONE)
        );
    }

    #[test]
    fn parses_modifiers() {
        assert_eq!(
            parse_chord("ctrl-alt-d").unwrap(),
            Chord::new(
                KeyCode::Char('d'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )
        );
    }

    #[test]
    fn parses_named_key() {
        assert_eq!(
            parse_chord("ctrl-alt-space").unwrap(),
            Chord::new(
                KeyCode::Char(' '),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )
        );
    }

    #[test]
    fn parses_trailing_hyphen_as_key() {
        assert_eq!(
            parse_chord("ctrl-alt--").unwrap(),
            Chord::new(
                KeyCode::Char('-'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )
        );
    }

    #[test]
    fn parses_bare_hyphen() {
        assert_eq!(
            parse_chord("-").unwrap(),
            Chord::new(KeyCode::Char('-'), KeyModifiers::NONE)
        );
    }

    #[test]
    fn normalizes_capital_forms() {
        let expected = Chord::new(KeyCode::Char('a'), KeyModifiers::SHIFT);
        assert_eq!(parse_chord("shift-a").unwrap(), expected);
        assert_eq!(parse_chord("A").unwrap(), expected);
        assert_eq!(parse_chord("shift-A").unwrap(), expected);
    }

    #[test]
    fn parses_function_keys() {
        assert_eq!(
            parse_chord("f12").unwrap(),
            Chord::new(KeyCode::F(12), KeyModifiers::NONE)
        );
    }

    #[test]
    fn rejects_unknown_modifier() {
        assert!(matches!(
            parse_chord("nope-d"),
            Err(ParseError::UnknownModifier(_))
        ));
    }

    #[test]
    fn rejects_unknown_key() {
        assert!(matches!(
            parse_chord("ctrl-nope"),
            Err(ParseError::UnknownKey(_))
        ));
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(parse_chord(""), Err(ParseError::Empty)));
    }
}
