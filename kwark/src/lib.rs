use std::{io::stdout, time::Duration};

use crossterm::event;
pub use kwark_input::{Chord, Step};

pub type InputState = kwark_input::InputState<State>;
pub type InputTree = kwark_input::InputTree<State>;

use ratatui::{
    macros::{horizontal, vertical},
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub mod prelude {
    pub use crate::Flags;
    pub use crate::InputState;
    pub use crate::InputTree;
    pub use crate::Running;
    pub use crate::State;

    pub use crossterm::event::{KeyCode, KeyModifiers};
    pub use kwark_input::{Chord, Step};

    pub use kwark_buffer as buffer;
    pub use kwark_text_buffer as text_buffer;
    pub use kwark_text_buffer_renderer as text_render;

    pub use crate::Pipeline;
}

use kwark_buffer as buffer;
pub type Pipeline = kwark_text_buffer_renderer::Pipeline<State>;

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
        self.state.insert(buffer::Storage::default());
        self.state.insert(Pipeline::default());
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
                            Step::Step => {
                                self.state.get::<&mut Flags>().set("input_tree_show", true)
                            }
                            Step::Failed => {
                                self.state.get::<&mut Flags>().set("input_tree_show", false)
                            }
                        };
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
                // Remove needed states from the storage
                let bufs = self.state.take::<buffer::Storage>();
                let mut pipeline = self.state.take::<Pipeline>();

                match bufs.iter().next() {
                    Some((_, buf)) => {
                        buf.render(&mut pipeline, &mut self.state, frame, frame.area())
                    }
                    None => {
                        frame.render_widget(
                            Paragraph::new("Kwark").centered().green(),
                            frame.area(),
                        );
                    }
                };

                // Re-insert the states
                self.state.insert(bufs);
                self.state.insert(pipeline);

                let (state, flags) = self.state.get::<(&InputState, &Flags)>();
                if flags.get("input_tree_show") {
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
