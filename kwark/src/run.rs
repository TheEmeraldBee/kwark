use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::event::{self, EventStream};
use futures_util::StreamExt;

use kwark_buffer as buffer;
use kwark_core::{
    Chord, Editor, EditorPipeline, Flags, InputState, MousePosition, Pipeline, Running, Runtime,
    Step,
    commands::CommandRegistry,
    events,
    frame::{FrameClock, FrameInfo, Redraw},
    log::{self, LogState},
    suspend::SuspendQueue,
    tasks::Tasks,
    timers::Timers,
};
use kwark_theme::Theme;
use kwark_window::{WindowLayout, WindowPipeline};

use crate::terminal;

use ratatui::{
    macros::{horizontal, vertical},
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// The flag that shows the input tree popup
const INPUT_TREE_FLAG: &str = "input_tree_show";

/// Time a chord sequence waits before the input tree popup shows
const INPUT_TREE_DELAY: Duration = Duration::from_millis(400);

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
    editor.state.insert(Redraw::default());
    editor.state.insert(FrameInfo::default());
    editor.state.insert(Timers::default());
    editor.state.insert(Tasks::default());
    editor.state.insert(SuspendQueue::default());

    editor
}

pub fn run(mut editor: Editor) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("async runtime should build");

    editor.state.insert(Runtime(runtime.handle().clone()));

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

    let res = runtime.block_on(run_inner(&mut editor));

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

async fn run_inner(editor: &mut Editor) -> anyhow::Result<()> {
    let mut input = EventStream::new();
    let mut clock = FrameClock::new(FRAME_INTERVAL);

    editor.state.redraw();

    while editor.state.get::<&Running>().0 {
        let frame_at = editor.state.get::<&Redraw>().deadline(clock.next_allowed());
        let timer_at = editor.state.get::<&mut Timers>().next_deadline();

        tokio::select! {
            biased;

            _ = sleep_until_opt(frame_at) => {
                let now = Instant::now();

                editor.state.get::<&mut Redraw>().clear(now);
                editor.state.get::<&mut FrameInfo>().advance(now);

                update_buffers(editor)?;
                draw_frame(editor)?;

                clock.presented(now);
            }
            event = input.next() => {
                let Some(event) = event else {
                    break;
                };

                handle_term_event(editor, event?);
            }
            Some(callback) = editor.recv_callback() => {
                let res = editor.handle_callback(callback);
                report(editor, res);

                editor.state.redraw();
            }
            _ = sleep_until_opt(timer_at) => fire_timers(editor, Instant::now()),
        }

        if !editor.state.get::<&SuspendQueue>().is_empty() {
            // The stream's reader would compete with the suspended program for keys
            drop(input);
            run_suspended(editor)?;
            input = EventStream::new();
        }
    }

    Ok(())
}

fn fire_timers(editor: &mut Editor, now: Instant) {
    while let Some(mut due) = editor.state.get::<&mut Timers>().pop_due(now) {
        let res = due.run(&mut editor.state);
        report(editor, res);

        editor.state.get::<&mut Timers>().finish(due);
    }

    editor.state.redraw();
}

fn run_suspended(editor: &mut Editor) -> anyhow::Result<()> {
    // Waits for the dropped stream's reader to release the terminal and clears its pending wake
    event::poll(Duration::ZERO)?;

    terminal::leave()?;

    for f in editor.state.get::<&mut SuspendQueue>().take() {
        let res = f(&mut editor.state);
        report(editor, res);
    }

    terminal::enter()?;

    // Blanks the previous frame so the next draw repaints every cell
    editor.term().swap_buffers();
    editor.state.redraw();

    Ok(())
}

/// Waits until the deadline, or forever without one
async fn sleep_until_opt(at: Option<Instant>) {
    match at {
        Some(at) => tokio::time::sleep_until(at.into()).await,
        None => std::future::pending().await,
    }
}

/// Logs an error from a handler
fn report(editor: &mut Editor, res: anyhow::Result<()>) {
    let Err(error) = res else {
        return;
    };

    if editor.has::<LogState>() {
        editor
            .get::<&mut LogState>()
            .log(error.to_string(), log::Severity::Error)
    }

    editor.state.redraw();
}

fn handle_term_event(editor: &mut Editor, event: event::Event) {
    let mut errors = vec![];

    let visible = match &event {
        event::Event::Key(_) | event::Event::Resize(..) => true,
        event::Event::Mouse(m) => m.kind != event::MouseEventKind::Moved,
        _ => false,
    };

    if visible {
        editor.state.redraw();
    }

    let chord = match event {
        event::Event::Key(k) => Some(Chord::from(k)),
        event::Event::Mouse(m) => {
            editor.state.insert(MousePosition {
                column: m.column,
                row: m.row,
            });

            match m.kind {
                event::MouseEventKind::Drag(event::MouseButton::Left) => {
                    if let Err(e) = editor
                        .events
                        .handle(&mut editor.state, &mut events::MouseDrag)
                    {
                        errors.push(e);
                    }
                }
                event::MouseEventKind::Up(event::MouseButton::Left) => {
                    if let Err(e) = editor
                        .events
                        .handle(&mut editor.state, &mut events::MouseUp)
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
                    hide_input_tree(editor);

                    if let Err(e) = c(&mut editor.state) {
                        errors.push(e)
                    }
                }
                Step::Step => {
                    editor.state.get::<&mut Timers>().debounce(
                        INPUT_TREE_FLAG,
                        INPUT_TREE_DELAY,
                        |s| {
                            s.get::<&mut Flags>().set(INPUT_TREE_FLAG, true);
                            Ok(())
                        },
                    );
                }
                Step::Failed => {
                    hide_input_tree(editor);

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

    for error in errors {
        report(editor, Err(error));
    }
}

fn hide_input_tree(editor: &mut Editor) {
    editor
        .state
        .get::<&mut Timers>()
        .cancel_key(INPUT_TREE_FLAG);
    editor.state.get::<&mut Flags>().set(INPUT_TREE_FLAG, false);
}

fn update_buffers(editor: &mut Editor) -> anyhow::Result<()> {
    let (bufs, store, windows) = editor.state.get::<(
        &mut buffer::Storage,
        &mut kwark_tree_sitter::store::LanguageStore,
        &mut WindowLayout,
    )>();

    bufs.update_all(store)?;
    windows.update_all(bufs);

    Ok(())
}

fn draw_frame(editor: &mut Editor) -> anyhow::Result<()> {
    editor.draw(|frame, state| {
        // Remove needed states from the storage
        let bufs = state.take::<buffer::Storage>();
        let mut pipeline = state.take::<Pipeline>();
        let mut window_pipeline = state.take::<WindowPipeline>();
        let mut windows = state.take::<WindowLayout>();
        let mut editor_pipeline = state.take::<EditorPipeline>();

        let grid_rect = editor_pipeline.render(state, &(), frame.area(), frame.buffer_mut());

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
        if flags.get(INPUT_TREE_FLAG) {
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
    })
}
