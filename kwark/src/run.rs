use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::event::{self, EventStream};
use futures_util::StreamExt;

use kwark_buffer as buffer;
use kwark_core::{
    Chord, Editor, EditorPipeline, Flags, INPUT_TREE_FLAG, InputState, MousePosition, Overlays,
    Pipeline, Running, Runtime, Step,
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

use crate::{syntax, terminal};

const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// Delay before the input tree popup shows
const INPUT_TREE_DELAY: Duration = Duration::from_millis(400);

/// Creates an editor with the default state
pub fn init() -> Editor {
    let mut editor = Editor::default();
    let bus = editor.state.bus();

    editor.state.insert(buffer::Storage::new(bus.clone()));
    editor.state.insert(Pipeline::default());
    editor.state.insert(EditorPipeline::default());
    editor.state.insert(Overlays::default());
    editor.state.insert(WindowPipeline::default());
    editor.state.insert(InputState::new("normal", bus.clone()));
    editor.state.insert(Running(true));
    editor.state.insert(Theme::default());
    editor.state.insert(WindowLayout::new(bus));
    editor.state.insert(MousePosition::default());
    editor.state.insert(CommandRegistry::default());
    editor.state.insert(LogState::default());
    editor.state.insert(Redraw::default());
    editor.state.insert(FrameInfo::default());
    editor.state.insert(Timers::default());
    editor.state.insert(Tasks::default());
    editor.state.insert(SuspendQueue::default());

    syntax::register(&mut editor);

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

                let info = *editor.state.get::<&FrameInfo>();
                let res = editor.events.handle(&mut editor.state, &mut events::Frame { info });
                report(editor, res);

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
            Some(incoming) = editor.recv() => {
                let res = editor.handle(incoming);
                report(editor, res);

                editor.state.redraw();
            }
            _ = sleep_until_opt(timer_at) => fire_timers(editor, Instant::now()),
        }

        if !editor.state.get::<&SuspendQueue>().is_empty() {
            // Frees terminal input for the suspended program
            drop(input);
            run_suspended(editor)?;
            input = EventStream::new();
        }

        editor.state.get::<&mut buffer::Storage>().report_changes();
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
    // Waits until terminal input is fully released
    event::poll(Duration::ZERO)?;

    terminal::leave()?;

    for f in editor.state.get::<&mut SuspendQueue>().take() {
        let res = f(&mut editor.state);
        report(editor, res);
    }

    terminal::enter()?;

    // Forces a full repaint on the next draw
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

    if let event::Event::Resize(width, height) = event
        && let Err(e) = editor
            .events
            .handle(&mut editor.state, &mut events::Resize { width, height })
    {
        errors.push(e);
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

        let mut overlays = state.take::<Overlays>();
        overlays.render(state, frame.area(), frame.buffer_mut());
        state.insert(overlays);
    })
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use kwark_core::prelude::text::BufferEntry;

    use super::*;

    #[test]
    pub fn handlers_see_events_from_other_handlers() {
        let mut editor = init();
        let log = Rc::new(RefCell::new(vec![]));

        let opened = log.clone();
        editor.events.on::<events::BufferOpened>(move |s, _| {
            opened.borrow_mut().push("opened");
            s.get::<&mut InputState>().set_mode("insert");
            Ok(())
        });

        let mode = log.clone();
        editor.events.on::<events::ModeChanged>(move |_, e| {
            mode.borrow_mut().push("mode");
            assert_eq!(e.to, "insert");
            Ok(())
        });

        editor
            .get::<&mut buffer::Storage>()
            .insert(BufferEntry::new_scratch("a"));

        while let Some(event) = editor.try_recv_event() {
            editor.handle_event(event).unwrap();
        }

        assert_eq!(*log.borrow(), ["opened", "mode"]);
    }
}
