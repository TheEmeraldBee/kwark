use std::io::{self, stdout};

use crossterm::{
    event,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

/// Hands the terminal back to the shell
pub(crate) fn leave() -> io::Result<()> {
    crossterm::execute!(
        stdout(),
        event::PopKeyboardEnhancementFlags,
        event::DisableBracketedPaste,
        event::DisableMouseCapture,
        LeaveAlternateScreen,
    )?;

    terminal::disable_raw_mode()
}

/// Takes the terminal back from the shell
pub(crate) fn enter() -> io::Result<()> {
    terminal::enable_raw_mode()?;

    crossterm::execute!(
        stdout(),
        event::PushKeyboardEnhancementFlags(
            event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        ),
        event::EnableBracketedPaste,
        event::EnableMouseCapture,
        EnterAlternateScreen,
        terminal::Clear(terminal::ClearType::All),
    )
}
