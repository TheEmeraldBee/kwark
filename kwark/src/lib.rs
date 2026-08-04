use std::{io::stdout, time::Duration};

use crossterm::event;
use kaon::engine::{Args, FunctionBuilder};
use kwark_buffer::BufferList;
use kwark_input::{Chord, InputState, Step, parse_chord};
use ratatui::{text::Line, widgets::Paragraph};

mod state;

pub use state::*;

pub mod events;

pub use kaon::prelude as lang;

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

        self.engine.register(
            "quit",
            FunctionBuilder::new()
                .desc("Quits the editor")
                .build(|args: &mut Args<'_, State>| {
                    args.cx().get::<&mut Running>().quit();
                    Ok(lang::Value::Null)
                }),
        );

        self.engine.namespace("buffer").register(
            "open",
            FunctionBuilder::new()
                .arg(
                    "path",
                    "the path to the file you want to open",
                    Some(lang::Type::Str),
                )
                .build(|args: &mut Args<'_, State>| {
                    let path = args.str("path")?;

                    let list = args.cx().get::<&mut BufferList>();
                    let id = list
                        .file(path.into())
                        .map_err(|e| kaon::error::Error::External(e.to_string()))?;

                    Ok(lang::Value::Int(id as i32))
                }),
        );

        self.engine.namespace("input")
            .register(
                "bind",
                FunctionBuilder::new()
                    .desc("Registers a bind for the given mode and key-sequence, and runs the given method on success")
                    .arg("mode", "The mode that this binding applies to", Some(lang::Type::Str))
                    .arg("chord", "The key presses that make up the keybind (list of strings)", Some(lang::Type::List))
                    .arg("event", "The function to run on execution (takes 0 arguments)", Some(lang::Type::Method))
                    .build(|args: &mut Args<'_, State>| {
                        let mode = args.str("mode")?;
                        let chord = args.mapped_list("chord", |v| v.str().map(str::to_string))?;
                        let event = args.method("event")?;

                        let chord = chord
                            .iter()
                            .map(|s| parse_chord(s))
                            .collect::<Result<Vec<Chord>, _>>()
                            .map_err(|e| kaon::error::Error::External(e.to_string()))?;

                        let cx = args.cx();
                        let input = cx.get::<&mut InputState>();

                        if event.0.len() != 0 {
                            return Err(lang::KaonError::External("expected 0 arguments to bind event".to_string()))
                        }

                        input.tree(mode).register(&chord, "hello", lang::Value::Method { args: event.0, body: event.1 });

                        Ok(lang::Value::Null)
            }))
            .register("backup",
                FunctionBuilder::new()
                    .desc("Sets the backup function for a given mode to a function that takes the chord")
                    .arg("mode", "The mode that this binding applies to", Some(lang::Type::Str))
                    .arg("backup", "The method to run on the backup (requires 1 argument)", Some(lang::Type::Method))
                    .build(|args: &mut Args<'_, State>| {
                        let mode = args.str("mode")?;
                        let event = args.method("backup")?;

                        let cx = args.cx();
                        let input = cx.get::<&mut InputState>();

                        if event.0.len() != 1 {
                            return Err(lang::KaonError::External("expected 1 argument to backup method".to_string()))
                        }

                        input.tree(mode).set_backup(lang::Value::Method { args: event.0, body: event.1 });

                        Ok(lang::Value::Null)
            }));
    }

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
                            Step::Complete(c, chords) => {
                                let method =
                                    c.method().expect("Value in input **should** be method");

                                self.scope.push_blocking_frame();
                                if let Some(param) = method.0.first() {
                                    let chord = chords
                                        .iter()
                                        .map(Chord::to_string)
                                        .collect::<Vec<_>>()
                                        .join(" ");
                                    self.scope.register(param.clone(), lang::Value::Str(chord));
                                }
                                let res = self.engine.solve(&method.1, &mut self.scope, &mut self.state);
                                self.scope.pop_frame();
                                res.expect("Should have worked");
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
                    .unwrap()
                    .viewport((0, 0), (100, 100))
                    .into_iter()
                    .map(|x| Line::raw(x.text))
                    .collect::<Vec<_>>();

                frame.render_widget(Paragraph::new(lines), frame.area());
            })?;
        }

        Ok(())
    }
}
