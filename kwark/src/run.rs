use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::event::{self, EventStream};
use futures_util::StreamExt;

use kwark_core::{
    Chord, Editor, INPUT_TREE_FLAG, MousePosition, Step, events,
    frame::FrameClock,
    log::{self},
};

use crate::terminal;

const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// Delay before the input tree popup shows
const INPUT_TREE_DELAY: Duration = Duration::from_millis(400);

/// Runs the editor until a binding quits it
pub fn run<S: 'static>(mut editor: Editor<S>) {
    let runtime = editor.runtime().clone();
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

async fn run_inner<S: 'static>(editor: &mut Editor<S>) -> anyhow::Result<()> {
    let mut input = EventStream::new();
    let mut clock = FrameClock::new(FRAME_INTERVAL);

    editor.redraw();

    while editor.core.running.0 {
        let frame_at = editor.core.redraw.deadline(clock.next_allowed());
        let timer_at = editor.core.timers.next_deadline();

        tokio::select! {
            biased;

            _ = sleep_until_opt(frame_at) => {
                let now = Instant::now();

                editor.core.redraw.clear(now);
                editor.core.frame.advance(now);

                let info = editor.core.frame;
                let res = editor.emit(&mut events::Frame { info });
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

                editor.redraw();
            }
            _ = sleep_until_opt(timer_at) => fire_timers(editor, Instant::now()),
        }

        if !editor.core.suspend.is_empty() {
            // Frees terminal input for the suspended program
            drop(input);
            run_suspended(editor)?;
            input = EventStream::new();
        }

        editor.core.buffers.report_changes();
    }

    Ok(())
}

fn fire_timers<S: 'static>(editor: &mut Editor<S>, now: Instant) {
    while let Some(mut due) = editor.core.timers.pop_due(now) {
        let res = due.run(editor);
        report(editor, res);

        editor.core.timers.finish(due);
    }

    editor.redraw();
}

fn run_suspended<S: 'static>(editor: &mut Editor<S>) -> anyhow::Result<()> {
    // Waits until terminal input is fully released
    event::poll(Duration::ZERO)?;

    terminal::leave()?;

    for f in editor.core.suspend.take() {
        let res = f(editor);
        report(editor, res);
    }

    terminal::enter()?;

    // Forces a full repaint on the next draw
    editor.term().swap_buffers();
    editor.redraw();

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
fn report<S: 'static>(editor: &mut Editor<S>, res: anyhow::Result<()>) {
    let Err(error) = res else {
        return;
    };

    editor.core.log.log(error.to_string(), log::Severity::Error);

    editor.redraw();
}

fn handle_term_event<S: 'static>(editor: &mut Editor<S>, event: event::Event) {
    let mut errors = vec![];

    let visible = match &event {
        event::Event::Key(_) | event::Event::Resize(..) => true,
        event::Event::Mouse(m) => m.kind != event::MouseEventKind::Moved,
        _ => false,
    };

    if visible {
        editor.redraw();
    }

    if let event::Event::Resize(width, height) = event
        && let Err(e) = editor.emit(&mut events::Resize { width, height })
    {
        errors.push(e);
    }

    let chord = match event {
        event::Event::Key(k) => Some(Chord::from(k)),
        event::Event::Mouse(m) => {
            editor.core.mouse = MousePosition {
                column: m.column,
                row: m.row,
            };

            match m.kind {
                event::MouseEventKind::Drag(event::MouseButton::Left) => {
                    if let Err(e) = editor.emit(&mut events::MouseDrag) {
                        errors.push(e);
                    }
                }
                event::MouseEventKind::Up(event::MouseButton::Left) => {
                    if let Err(e) = editor.emit(&mut events::MouseUp) {
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

        if let Err(e) = editor.emit(&mut e) {
            errors.push(e)
        }

        editor.core.log.clear();

        // Allow the highest level to block input state
        if !e.block {
            match editor.core.input.step(chord) {
                Step::Complete(c, _chords) => {
                    hide_input_tree(editor);

                    if let Err(e) = c(editor) {
                        errors.push(e)
                    }
                }
                Step::Step => {
                    editor
                        .core
                        .timers
                        .debounce(INPUT_TREE_FLAG, INPUT_TREE_DELAY, |e| {
                            e.core.flags.set(INPUT_TREE_FLAG, true);
                            Ok(())
                        });
                }
                Step::Backup(backup, chord) => {
                    hide_input_tree(editor);

                    match backup(editor, chord) {
                        Ok(true) => {}
                        Ok(false) => {
                            if let Err(e) = editor.emit(&mut events::ExtraInput { chord }) {
                                errors.push(e)
                            }
                        }
                        Err(e) => errors.push(e),
                    }
                }
                Step::Failed => {
                    hide_input_tree(editor);

                    // Nothing happened, so handle the remaining tooling
                    if let Err(e) = editor.emit(&mut events::ExtraInput { chord }) {
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

fn hide_input_tree<S: 'static>(editor: &mut Editor<S>) {
    editor.core.timers.cancel_key(INPUT_TREE_FLAG);
    editor.core.flags.set(INPUT_TREE_FLAG, false);
}

fn update_buffers<S: 'static>(editor: &mut Editor<S>) -> anyhow::Result<()> {
    let core = &mut editor.core;

    core.buffers.update_all(&mut core.languages)?;
    core.windows.update_all(&core.buffers);

    Ok(())
}

fn draw_frame<S: 'static>(editor: &mut Editor<S>) -> anyhow::Result<()> {
    editor.draw(|frame, editor| {
        editor.with_view(|v, editor| {
            let area = frame.area();

            let grid_rect = v
                .editor_pipeline
                .render(editor, &(), area, frame.buffer_mut());

            v.windows.render(
                &v.buffers,
                frame,
                grid_rect,
                &mut v.pipeline,
                &mut v.window_pipeline,
                editor,
            );

            v.overlays.render(editor, area, frame.buffer_mut());
        })
    })
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use kwark_core::prelude::text::BufferEntry;

    use super::*;
    use crate::plugins;

    #[crate::state]
    struct Empty {}

    #[crate::state]
    struct Full {
        #[plugin]
        visual: plugins::visual::Plugin,
        #[plugin]
        editor_commands: plugins::editor_commands::Plugin,
        #[plugin]
        movement: plugins::movement::Plugin,
        #[plugin]
        insert: plugins::insert::Plugin,
        #[plugin]
        window: plugins::window::Plugin,
        #[plugin]
        command_line: plugins::command_line::Plugin,
        #[plugin(plugins::smooth_scroll::Plugin::new(40.0))]
        smooth_scroll: plugins::smooth_scroll::Plugin,
    }

    /// Runs every queued bus event and callback
    fn drain<S: 'static>(editor: &mut Editor<S>) {
        loop {
            if let Some(event) = editor.try_recv_event() {
                editor.handle(kwark_core::Incoming::Event(event)).unwrap();
            } else if let Some(callback) = editor.try_recv_callback() {
                editor.handle_callback(callback).unwrap();
            } else {
                break;
            }
        }
    }

    #[test]
    pub fn handlers_see_events_from_other_handlers() {
        let mut editor = crate::init::<Empty>().unwrap();
        let log = Rc::new(RefCell::new(vec![]));

        let opened = log.clone();
        editor.events.on::<events::BufferOpened>(move |e, _| {
            opened.borrow_mut().push("opened");
            e.core.input.set_mode("insert");
            Ok(())
        });

        let mode = log.clone();
        editor.events.on::<events::ModeChanged>(move |_, e| {
            mode.borrow_mut().push("mode");
            assert_eq!(e.to, "insert");
            Ok(())
        });

        editor.core.buffers.insert(BufferEntry::new_scratch("a"));

        drain(&mut editor);

        assert_eq!(*log.borrow(), ["opened", "mode"]);
    }

    #[test]
    pub fn every_plugin_boots_and_runs_commands() {
        let mut editor = crate::init::<Full>().unwrap();

        let id = editor.core.buffers.insert(BufferEntry::new_scratch("a"));
        editor.core.windows.active_mut().select(id);
        drain(&mut editor);

        editor.execute("insert text=\"hello\"").unwrap();
        editor.execute("move dir=left count=2").unwrap();
        editor.execute("split dir=right").unwrap();
        editor.execute("focus dir=left").unwrap();
        editor.execute("toggle_all_cursors").unwrap();
        drain(&mut editor);

        assert!(editor.core.cursor.all);
        assert!(editor.execute("unknown_command").is_err());

        plugins::command_line::command_mode(&mut editor);
        assert_eq!(editor.core.input.mode(), "command");

        editor.execute("quit").unwrap();
        assert!(!editor.core.running.0);
    }

    #[test]
    pub fn smooth_scroll_drops_closed_windows() {
        let mut editor = crate::init::<Full>().unwrap();

        let id = editor.core.buffers.insert(BufferEntry::new_scratch("a"));
        editor.core.windows.active_mut().select(id);
        editor.execute("split dir=right").unwrap();
        drain(&mut editor);

        let info = editor.core.frame;
        editor.emit(&mut events::Frame { info }).unwrap();
        assert_eq!(editor.plugin::<plugins::smooth_scroll::Plugin>().len(), 2);

        editor.execute("close_window").unwrap();
        editor.emit(&mut events::Frame { info }).unwrap();
        assert_eq!(editor.plugin::<plugins::smooth_scroll::Plugin>().len(), 1);
    }
}
