mod state;

pub mod commands;
pub mod events;
pub mod layout;
pub mod pipelines;
pub mod plugins;
pub mod prelude;

pub use state::*;

use std::{io::stdout, time::Duration};

use crossterm::{
    event,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};

use kwark_buffer as buffer;
use kwark_theme::Theme;

use ratatui::{
    macros::{horizontal, vertical},
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

use commands::CommandRegistry;

pub use kwark_input::{Chord, Code, Step};

pub type InputState = kwark_input::InputState<State>;
pub type InputTree = kwark_input::InputTree<State>;
pub type Pipeline = kwark_text_buffer_renderer::Pipeline<State>;
pub type WindowPipeline = kwark_text_buffer_renderer::WindowPipeline<State, layout::WindowCtx>;
pub type EditorPipeline = kwark_text_buffer_renderer::EditorPipeline<State, ()>;

pub struct Running(pub bool);

impl Running {
    pub fn quit(&mut self) {
        self.0 = false
    }
}

/// The screen position of the most recent mouse event, read by chords bound to a mouse press
#[derive(Default, Clone, Copy)]
pub struct MousePosition {
    pub column: u16,
    pub row: u16,
}

pub fn init() -> Editor {
    let mut editor = Editor::default();
    editor.init();
    editor
}

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

/// An "installable" set of tooling to add to the editor
pub trait Plugin {
    fn register(&self, editor: &mut Editor) -> anyhow::Result<()>;
}

impl Editor {
    pub fn init(&mut self) {
        self.state.insert(buffer::Storage::default());
        self.state.insert(Pipeline::default());
        self.state.insert(EditorPipeline::default());
        self.state.insert(WindowPipeline::default());
        self.state.insert(InputState::new("normal"));
        self.state.insert(Running(true));
        self.state.insert(Theme::default());
        self.state.insert(layout::WindowLayout::new());
        self.state.insert(MousePosition::default());
        self.state.insert(CommandRegistry::default());
    }

    /// Installs a plugin
    pub fn install(&mut self, plugin: impl Plugin) -> anyhow::Result<()> {
        plugin.register(self)
    }

    pub fn run(mut self) {
        let term = ratatui::init();
        crossterm::execute!(
            stdout(),
            event::PushKeyboardEnhancementFlags(
                event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            ),
            event::EnableBracketedPaste,
            event::EnableMouseCapture,
        )
        .unwrap();

        self.term = Some(term);

        let res = self.run_inner();

        crossterm::execute!(
            stdout(),
            event::PopKeyboardEnhancementFlags,
            event::DisableBracketedPaste,
            event::DisableMouseCapture,
        )
        .unwrap();
        ratatui::restore();

        res.unwrap()
    }

    fn run_inner(&mut self) -> anyhow::Result<()> {
        while self.state.get::<&Running>().0 {
            // Force minimum of 60 fps
            std::thread::sleep(Duration::from_secs_f32(1.0 / 60.0));

            // Collect all events
            let events = match crossterm::event::poll(Duration::from_millis(150))? {
                true => {
                    let mut out = vec![crossterm::event::read()?];
                    while crossterm::event::poll(Duration::ZERO)? {
                        out.push(crossterm::event::read()?);
                    }
                    out
                }
                false => vec![],
            };

            // Handle all events
            for event in events {
                let chord = match event {
                    event::Event::Key(k) => Some(Chord::from(k)),
                    event::Event::Mouse(m) => {
                        self.state.insert(MousePosition {
                            column: m.column,
                            row: m.row,
                        });

                        match m.kind {
                            event::MouseEventKind::Drag(event::MouseButton::Left) => {
                                self.events
                                    .handle(&mut self.state, &mut events::MouseDrag)?;
                            }
                            event::MouseEventKind::Up(event::MouseButton::Left) => {
                                self.events.handle(&mut self.state, &mut events::MouseUp)?;
                            }
                            _ => {}
                        }

                        Chord::try_from(m).ok()
                    }
                    _ => None,
                };

                if let Some(chord) = chord {
                    let mut e = events::Input {
                        chord,
                        block: false,
                    };

                    self.events.handle(&mut self.state, &mut e)?;

                    // Allow the highest level to block input state
                    if !e.block {
                        match self.state.get::<&mut InputState>().step(chord) {
                            Step::Complete(c, _chords) => {
                                self.state.get::<&mut Flags>().set("input_tree_show", false);
                                c(&mut self.state)?;
                            }
                            Step::Step => {
                                self.state.get::<&mut Flags>().set("input_tree_show", true)
                            }
                            Step::Failed => {
                                self.state.get::<&mut Flags>().set("input_tree_show", false);

                                // Nothing happened, so handle the remaining tooling
                                self.events
                                    .handle(&mut self.state, &mut events::ExtraInput { chord })?;
                            }
                        };
                    }
                }
            }

            self.flush()?;

            {
                let (bufs, store, windows) = self.state.get::<(
                    &mut buffer::Storage,
                    &mut kwark_tree_sitter::store::LanguageStore,
                    &mut layout::WindowLayout,
                )>();

                bufs.update_all(store)?;
                windows.update_all(bufs);
            }

            let mut term = self.take_term();

            term.draw(|frame| {
                // Remove needed states from the storage
                let bufs = self.state.take::<buffer::Storage>();
                let mut pipeline = self.state.take::<Pipeline>();
                let mut window_pipeline = self.state.take::<WindowPipeline>();
                let mut windows = self.state.take::<layout::WindowLayout>();
                let mut editor_pipeline = self.state.take::<EditorPipeline>();

                let grid_rect =
                    editor_pipeline.render(&mut self.state, &(), frame.area(), frame.buffer_mut());

                windows.render(
                    &bufs,
                    frame,
                    grid_rect,
                    &mut pipeline,
                    &mut window_pipeline,
                    &mut self.state,
                );

                // Re-insert the states
                self.state.insert(bufs);
                self.state.insert(pipeline);
                self.state.insert(window_pipeline);
                self.state.insert(windows);
                self.state.insert(editor_pipeline);

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

                    // Clear the area under the widget
                    frame.render_widget(Clear, rect);

                    // Render the actual widget
                    frame.render_widget(
                        Paragraph::new(lines).block(Block::new().borders(Borders::ALL)),
                        rect,
                    );
                }
            })?;

            self.set_term(term);
        }

        Ok(())
    }
}
