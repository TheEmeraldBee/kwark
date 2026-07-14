use std::{io::stdout, time::Duration};

use crossterm::event;
use ratatui::widgets;

mod state;

pub use state::*;

mod events;

pub fn init() -> Editor {
    Editor::default()
}

impl Editor {
    pub fn run(mut self) {
        let term = ratatui::init();
        crossterm::execute!(
            stdout(),
            event::PushKeyboardEnhancementFlags(
                event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | event::KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                    | event::KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
            ),
            event::EnableBracketedPaste
        )
        .unwrap();

        let res = self.run_inner(term);

        crossterm::execute!(
            stdout(),
            event::PopKeyboardEnhancementFlags,
            event::DisableBracketedPaste
        )
        .unwrap();
        ratatui::restore();

        res.unwrap()
    }

    fn run_inner(&mut self, mut term: ratatui::DefaultTerminal) -> anyhow::Result<()> {
        let mut text = "Kwark".to_string();

        loop {
            let event = match crossterm::event::poll(Duration::from_millis(50))? {
                true => Some(crossterm::event::read()?),
                false => None,
            };

            if let Some(event) = event {
                match event {
                    event::Event::FocusGained => {}
                    event::Event::FocusLost => {}
                    event::Event::Key(k) => {
                        if k.is_press() && k.code == event::KeyCode::Char('q') {
                            return Ok(());
                        }
                        self.events.handle(&mut self.state, &events::Input(k))?;
                    }
                    event::Event::Paste(pasted) => text = pasted,
                    _ => {}
                }
            }

            self.flush()?;

            term.draw(|frame| {
                frame.render_widget(
                    widgets::Paragraph::new(text.as_str()).centered(),
                    frame.area(),
                );
            })?;
        }
    }
}
