use std::io::stdout;

use crossterm::{
    event,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};

use kwark_core::CallbackSender;

/// Run a function, replacing the terminal with this
///
/// Then auto-restore the window
pub fn replace<R>(
    mut sender: CallbackSender,
    func: impl FnOnce() -> anyhow::Result<R>,
) -> anyhow::Result<R> {
    crossterm::execute!(
        stdout(),
        event::PopKeyboardEnhancementFlags,
        event::DisableBracketedPaste,
        event::DisableMouseCapture,
        LeaveAlternateScreen,
    )
    .unwrap();

    let res = func();

    crossterm::execute!(
        stdout(),
        event::PushKeyboardEnhancementFlags(
            event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        ),
        event::EnableBracketedPaste,
        event::EnableMouseCapture,
        EnterAlternateScreen
    )
    .unwrap();

    // Tell the terminal to do a full redraw
    sender.send_fn(|e| {
        e.term().clear()?;
        Ok(())
    });

    res
}
