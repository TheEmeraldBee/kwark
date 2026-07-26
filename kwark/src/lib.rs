use std::{io::stdout, time::Duration};

use crossterm::event;
use kaon::engine::{Args, FunctionBuilder};
use kwark_buffer::{Buffer, BufferList};
use ratatui::{text::Line, widgets::Paragraph};

mod state;

pub use state::*;

mod events;

pub use kaon::prelude as lang;

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
        self.state.insert(BufferList::default());

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
                    list.file(path.into())
                        .map_err(|e| kaon::error::Error::External(e.to_string()))?;

                    Ok(lang::Value::Null)
                }),
        );

        self.exec(r#"buffer::open("./Cargo.toml")"#).unwrap();

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
                        self.events.handle(&mut self.state, &mut events::Input(k))?;
                    }
                    _ => {}
                }
            }

            self.flush()?;

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
    }
}
