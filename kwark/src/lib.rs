use std::{io::stdout, time::Duration};

use crossterm::event;
use kwark_buffer::BufferList;
pub use kwark_input::{Chord, Step};
pub type InputState = kwark_input::InputState<State>;
use ratatui::{
    macros::{horizontal, vertical},
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub mod prelude {
    pub use crate::InputState;
    pub use crate::Running;
    pub use crate::State;
    pub use crossterm::event::{KeyCode, KeyModifiers};
    pub use kwark_buffer::*;
    pub use kwark_input::{Chord, Step};
}

mod state;

pub use state::*;

pub mod events;

pub struct Running(pub bool);

impl Running {
    pub fn quit(&mut self) {
        self.0 = false
    }
}

pub fn init() -> Editor {
    let mut editor = Editor::default();
    editor.init();
    editor
}

impl Editor {
    pub fn init(&mut self) {
        self.state.insert(BufferList::default());
        self.state.insert(InputState::new("normal"));
        self.state.insert(Running(true));
    }

    pub fn run(mut self) {
        let term = ratatui::init();
        crossterm::execute!(
            stdout(),
            event::PushKeyboardEnhancementFlags(
                event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES // | event::KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                                                                           // | event::KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
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
        while self.state.get::<&Running>().0 {
            let event = match crossterm::event::poll(Duration::from_millis(50))? {
                true => Some(crossterm::event::read()?),
                false => None,
            };

            if let Some(event) = event {
                match event {
                    event::Event::FocusGained => {}
                    event::Event::FocusLost => {}
                    event::Event::Key(k) => {
                        match self.state.get::<&mut InputState>().step(Chord::from(k)) {
                            Step::Complete(c, _chords) => {
                                c(&mut self.state)?;
                            }
                            _ => {}
                        };
                        // self.events.handle(&mut self.state, &mut events::Input(k))?;
                    }
                    _ => {}
                }
            }

            self.flush()?;

            // TODO: The Window:
            // TODO: The window is made up of 3 major parts
            // TODO: ---- TOP BAR ----
            // TODO: WINDOWS & tab-bar
            // TODO: --- BOTTOM BAR --
            // TODO:
            // TODO: The Bottom Bar is a set of text in the left, middle, and right that can be changed
            // TODO: The Windows are either terminals or buffers that are rendered, each window also gets a tab-bar
            // TODO: The Top Bar is also a set of left, middle, and right text that is fully customizable

            term.draw(|frame| {
                let bufs = self.state.get::<&mut BufferList>();

                let lines = bufs
                    .get(0)
                    .map(|buf| buf.viewport((0, 0), (100, 100)))
                    .unwrap_or_default()
                    .into_iter()
                    .map(|x| Line::raw(x.text))
                    .collect::<Vec<_>>();

                frame.render_widget(Paragraph::new(lines), frame.area());

                let state = self.state.get::<&InputState>();
                if state.is_active() || true {
                    let lines = state
                        .get_layer()
                        .iter()
                        .map(|(chord, desc)| {
                            Line::raw(format!(
                                "{} : {desc}",
                                chord.map(|x| x.to_string()).unwrap_or("any".to_string())
                            ))
                        })
                        .collect::<Vec<_>>();

                    let vertical_layer = vertical![==80%, ==20%].split(frame.area())[1];
                    let rect = horizontal![==60%, ==40%].split(vertical_layer)[1];

                    frame.render_widget(
                        Paragraph::new(lines).block(Block::new().borders(Borders::ALL)),
                        rect,
                    );
                }
            })?;
        }

        Ok(())
    }
}
