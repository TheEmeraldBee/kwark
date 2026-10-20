use std::{io::stdout, time::Duration};

use crossterm::event;

use kwark_buffer as buffer;
use kwark_core::{
    Chord, Editor, EditorPipeline, Flags, InputState, MousePosition, Pipeline, Running, Step,
    commands::CommandRegistry,
    events,
    log::{self, LogState},
};
use kwark_theme::Theme;
use kwark_window::{WindowLayout, WindowPipeline};

use ratatui::{
    macros::{horizontal, vertical},
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Creates an editor with the default state
pub fn init() -> Editor {
    let mut editor = Editor::default();

    editor.state.insert(buffer::Storage::default());
    editor.state.insert(Pipeline::default());
    editor.state.insert(EditorPipeline::default());
    editor.state.insert(WindowPipeline::default());
    editor.state.insert(InputState::new("normal"));
    editor.state.insert(Running(true));
    editor.state.insert(Theme::default());
    editor.state.insert(WindowLayout::new());
    editor.state.insert(MousePosition::default());
    editor.state.insert(CommandRegistry::default());
    editor.state.insert(LogState::default());

    editor
}

pub fn run(mut editor: Editor) {
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

    editor.set_term(term);

    let res = run_inner(&mut editor);

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

fn run_inner(editor: &mut Editor) -> anyhow::Result<()> {
    let mut errors = vec![];

    while editor.state.get::<&Running>().0 {
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
                    editor.state.insert(MousePosition {
                        column: m.column,
                        row: m.row,
                    });

                    match m.kind {
                        event::MouseEventKind::Drag(event::MouseButton::Left) => {
                            if let Err(e) =
                                editor.events.handle(&mut editor.state, &mut events::MouseDrag)
                            {
                                errors.push(e);
                            }
                        }
                        event::MouseEventKind::Up(event::MouseButton::Left) => {
                            if let Err(e) =
                                editor.events.handle(&mut editor.state, &mut events::MouseUp)
                            {
                                errors.push(e);
                            }
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

                if let Err(e) = editor.events.handle(&mut editor.state, &mut e) {
                    errors.push(e)
                }

                editor.state.get::<&mut LogState>().clear();

                // Allow the highest level to block input state
                if !e.block {
                    match editor.state.get::<&mut InputState>().step(chord) {
                        Step::Complete(c, _chords) => {
                            editor.state.get::<&mut Flags>().set("input_tree_show", false);

                            if let Err(e) = c(&mut editor.state) {
                                errors.push(e)
                            }
                        }
                        Step::Step => {
                            editor.state.get::<&mut Flags>().set("input_tree_show", true)
                        }
                        Step::Failed => {
                            editor.state.get::<&mut Flags>().set("input_tree_show", false);

                            // Nothing happened, so handle the remaining tooling
                            if let Err(e) = editor
                                .events
                                .handle(&mut editor.state, &mut events::ExtraInput { chord })
                            {
                                errors.push(e)
                            }
                        }
                    };
                }
            }
        }

        editor.flush()?;

        {
            let (bufs, store, windows) = editor.state.get::<(
                &mut buffer::Storage,
                &mut kwark_tree_sitter::store::LanguageStore,
                &mut WindowLayout,
            )>();

            bufs.update_all(store)?;
            windows.update_all(bufs);
        }

        editor.draw(|frame, state| {
            // Remove needed states from the storage
            let bufs = state.take::<buffer::Storage>();
            let mut pipeline = state.take::<Pipeline>();
            let mut window_pipeline = state.take::<WindowPipeline>();
            let mut windows = state.take::<WindowLayout>();
            let mut editor_pipeline = state.take::<EditorPipeline>();

            let grid_rect =
                editor_pipeline.render(state, &(), frame.area(), frame.buffer_mut());

            windows.render(
                &bufs,
                frame,
                grid_rect,
                &mut pipeline,
                &mut window_pipeline,
                state,
            );

            // Re-insert the states
            state.insert(bufs);
            state.insert(pipeline);
            state.insert(window_pipeline);
            state.insert(windows);
            state.insert(editor_pipeline);

            let (state, flags) = state.get::<(&InputState, &Flags)>();
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

        for error in errors.drain(..) {
            if editor.has::<LogState>() {
                editor.get::<&mut LogState>()
                    .log(error.to_string(), log::Severity::Error)
            }
        }
    }

    Ok(())
}
