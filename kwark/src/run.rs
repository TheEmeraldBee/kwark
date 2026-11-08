use std::time::{Duration, Instant};

use kui::{App, Cx, Event, Key, KeyEvent, Mods, NamedKey, Node, Proxy, ScrollDelta, WindowOptions};

use kwark_core::{
    Chord, Code, Editor, INPUT_TREE_FLAG, Incoming, KeyCode, KeyModifiers, MouseButton, Step,
    events, log,
};

/// Delay before the input tree popup shows
const INPUT_TREE_DELAY: Duration = Duration::from_millis(400);

/// Runs the editor until a binding quits it or its window closes
pub fn run<S: 'static>(editor: Editor<S>) {
    if let Err(e) = kui::run(Kwark::new(editor)) {
        eprintln!("kwark: {e}");
        std::process::exit(1);
    }
}

/// The editor as an app that kui drives
struct Kwark<S: 'static> {
    editor: Editor<S>,
    /// The next time a timer or a delayed frame is due
    next_wake: Option<Instant>,
    /// Whether the left button is down, so that moves count as drags
    dragging: bool,
    /// Scroll from a touchpad that has not added up to a whole line yet
    scroll_rest: f32,
    /// Text from the last key press, so that the same text from the input method is not typed twice
    key_text: Option<String>,
}

impl<S: 'static> Kwark<S> {
    fn new(editor: Editor<S>) -> Self {
        Self {
            editor,
            next_wake: None,
            dragging: false,
            scroll_rest: 0.0,
            key_text: None,
        }
    }

    /// Hands editor requests to kui after each handler
    fn sync(&mut self, cx: &mut Cx) {
        let core = &mut self.editor.core;
        core.buffers.report_changes();

        if !core.running.0 {
            cx.quit();
            return;
        }

        let now = Instant::now();
        let frame_at = core.redraw.deadline(now);
        if frame_at.is_some_and(|at| at <= now) {
            cx.request_redraw();
        }

        let later_frame = frame_at.filter(|&at| at > now);
        self.next_wake = match (later_frame, core.timers.next_deadline()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
    }

    fn input(&mut self, event: Event, cx: &mut Cx) {
        let editor = &mut self.editor;
        let mods = key_mods(cx.mods());

        let chord = match event {
            Event::Key(key) => {
                self.key_text = key.text.clone();
                editor.redraw();
                chord_for_key(&key)
            }
            Event::Text(text) => {
                // Text that the last key press already typed
                if self.key_text.take().as_deref() == Some(text.as_str()) {
                    return;
                }

                editor.redraw();
                for c in text.chars() {
                    let chord = Chord::new(Code::Key(KeyCode::Char(c)), KeyModifiers::NONE);
                    handle_chord(editor, chord);
                }
                return;
            }
            Event::MouseDown { position, button } => {
                editor.core.mouse = position;
                editor.redraw();

                if button == kui::MouseButton::Left {
                    self.dragging = true;
                }

                mouse_button(button).map(|b| Chord::new(Code::Mouse(b), mods))
            }
            Event::MouseUp { position, button } => {
                editor.core.mouse = position;

                if button == kui::MouseButton::Left {
                    self.dragging = false;
                    editor.redraw();
                    emit(editor, &mut events::MouseUp);
                }
                None
            }
            Event::MouseMove { position } => {
                editor.core.mouse = position;

                if self.dragging {
                    editor.redraw();
                    emit(editor, &mut events::MouseDrag);
                }
                None
            }
            Event::Scroll { position, delta } => {
                editor.core.mouse = position;
                editor.redraw();

                let lines = match delta {
                    ScrollDelta::Lines(p) => p.y,
                    ScrollDelta::Pixels(p) => {
                        self.scroll_rest += p.y / editor.core.pipeline.metrics.line_height;
                        let whole = self.scroll_rest.trunc();
                        self.scroll_rest -= whole;
                        whole
                    }
                };

                let code = if lines > 0.0 {
                    Code::ScrollUp
                } else {
                    Code::ScrollDown
                };
                for _ in 0..lines.abs().round() as usize {
                    handle_chord(editor, Chord::new(code, mods));
                }
                return;
            }
            Event::Resize(size) => {
                editor.redraw();
                emit(
                    editor,
                    &mut events::Resize {
                        width: size.width,
                        height: size.height,
                    },
                );
                None
            }
            _ => None,
        };

        if let Some(chord) = chord {
            handle_chord(editor, chord);
        }
    }
}

impl<S: 'static> App for Kwark<S> {
    type State = Editor<S>;
    type Message = Incoming<S>;

    fn window(&self) -> WindowOptions {
        self.editor.core.window_config.clone()
    }

    fn init(&mut self, proxy: Proxy<Incoming<S>>, cx: &mut Cx) {
        // The window's fonts take over, keeping the rendering the flavor chose
        let pipeline = &mut self.editor.core.pipeline;
        cx.fonts().set_rendering(pipeline.metrics.fonts.rendering());
        pipeline.metrics = pipeline.metrics.with_fonts(cx.fonts().clone());

        if let Some(mut inbox) = self.editor.take_inbox() {
            self.editor.runtime().spawn(async move {
                while let Some(incoming) = inbox.recv().await {
                    if proxy.send(incoming).is_err() {
                        break;
                    }
                }
            });
        }

        self.editor.redraw();
        self.sync(cx);
    }

    fn state(&mut self) -> &mut Editor<S> {
        &mut self.editor
    }

    fn event(&mut self, event: Event, cx: &mut Cx) {
        self.input(event, cx);
        self.sync(cx);
    }

    fn message(&mut self, incoming: Incoming<S>, cx: &mut Cx) {
        let res = self.editor.handle(incoming);
        report(&mut self.editor, res);

        self.editor.redraw();
        self.sync(cx);
    }

    fn deadline(&self) -> Option<Instant> {
        self.next_wake
    }

    fn wake(&mut self, now: Instant, cx: &mut Cx) {
        let editor = &mut self.editor;

        while let Some(mut due) = editor.core.timers.pop_due(now) {
            let res = due.run(editor);
            report(editor, res);

            editor.core.timers.finish(due);
            editor.redraw();
        }

        self.sync(cx);
    }

    fn view(&mut self, cx: &mut Cx) -> Node<Editor<S>> {
        let editor = &mut self.editor;
        let now = Instant::now();

        editor.core.redraw.clear(now);
        editor.core.frame.advance(now);

        let info = editor.core.frame;
        let res = editor.emit(&mut events::Frame { info });
        report(editor, res);

        let core = &mut editor.core;
        let res = core.buffers.update_all(&mut core.languages);
        report(editor, res.map_err(Into::into));
        editor.core.windows.update_all(&editor.core.buffers);

        let text = editor.core.pipeline.metrics.text.clone();
        let color = editor.core.theme.get("text").fg.unwrap_or(text.color);
        let root = editor.view(cx.size());

        self.sync(cx);

        root.font(text.family).size(text.size).color(color)
    }
}

/// Returns the chord for a key press, None for keys that bind to nothing
fn chord_for_key(key: &KeyEvent) -> Option<Chord> {
    let code = match key.key {
        Key::Char(c) => KeyCode::Char(c),
        Key::Named(named) => match named {
            NamedKey::Enter => KeyCode::Enter,
            NamedKey::Tab => KeyCode::Tab,
            NamedKey::Backspace => KeyCode::Backspace,
            NamedKey::Delete => KeyCode::Delete,
            NamedKey::Insert => KeyCode::Insert,
            NamedKey::Escape => KeyCode::Esc,
            NamedKey::Up => KeyCode::Up,
            NamedKey::Down => KeyCode::Down,
            NamedKey::Left => KeyCode::Left,
            NamedKey::Right => KeyCode::Right,
            NamedKey::Home => KeyCode::Home,
            NamedKey::End => KeyCode::End,
            NamedKey::PageUp => KeyCode::PageUp,
            NamedKey::PageDown => KeyCode::PageDown,
            NamedKey::F(n) => KeyCode::F(n),
        },
        Key::Unknown => return None,
    };

    let mut mods = key_mods(key.mods);

    // Shift already picked the character, as with `:` from `;`, so bindings name the character alone
    if let KeyCode::Char(c) = code
        && !c.is_alphabetic()
        && !c.is_whitespace()
    {
        mods.remove(KeyModifiers::SHIFT);
    }

    Some(Chord::new(Code::Key(code), mods))
}

fn key_mods(mods: Mods) -> KeyModifiers {
    let mut out = KeyModifiers::NONE;

    for (held, flag) in [
        (mods.shift, KeyModifiers::SHIFT),
        (mods.ctrl, KeyModifiers::CONTROL),
        (mods.alt, KeyModifiers::ALT),
        (mods.logo, KeyModifiers::SUPER),
    ] {
        if held {
            out |= flag;
        }
    }

    out
}

fn mouse_button(button: kui::MouseButton) -> Option<MouseButton> {
    match button {
        kui::MouseButton::Left => Some(MouseButton::Left),
        kui::MouseButton::Right => Some(MouseButton::Right),
        kui::MouseButton::Middle => Some(MouseButton::Middle),
        _ => None,
    }
}

/// Runs a chord through the input event and the input tree
fn handle_chord<S: 'static>(editor: &mut Editor<S>, chord: Chord) {
    let mut e = events::Input {
        chord,
        block: false,
    };

    let mut errors = vec![];

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

    for error in errors {
        report(editor, Err(error));
    }
}

/// Emits an event, logging any error
fn emit<S: 'static, E: 'static>(editor: &mut Editor<S>, event: &mut E) {
    let res = editor.emit(event);
    report(editor, res);
}

/// Logs an error from a handler
fn report<S: 'static>(editor: &mut Editor<S>, res: anyhow::Result<()>) {
    let Err(error) = res else {
        return;
    };

    editor.core.log.log(error.to_string(), log::Severity::Error);

    editor.redraw();
}

fn hide_input_tree<S: 'static>(editor: &mut Editor<S>) {
    editor.core.timers.cancel_key(INPUT_TREE_FLAG);
    editor.core.flags.set(INPUT_TREE_FLAG, false);
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

    fn press(key: kui::Key, shift: bool) -> Option<Chord> {
        chord_for_key(&KeyEvent {
            key,
            text: None,
            mods: Mods {
                shift,
                ..Mods::default()
            },
            repeat: false,
        })
    }

    #[test]
    pub fn shifted_keys_match_their_bindings() {
        let chord = |c, mods| Some(Chord::new(Code::Key(KeyCode::Char(c)), mods));
        let none = KeyModifiers::NONE;
        let shift = KeyModifiers::SHIFT;

        assert_eq!(press(kui::Key::Char(':'), true), chord(':', none));
        assert_eq!(press(kui::Key::Char('!'), true), chord('!', none));
        assert_eq!(press(kui::Key::Char('A'), true), chord('a', shift));
        assert_eq!(press(kui::Key::Char(' '), true), chord(' ', shift));
        assert_eq!(press(kui::Key::Char(';'), false), chord(';', none));
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
