use std::time::Instant;

use crossterm::event::MouseButton;
use kwark_text::CursorOptions;
use ratatui::layout::Position;

use kwark_core::{CallbackSender, events, prelude::*};
use kwark_window::{LeafId, WindowLayout};

type ClickHandler = Box<dyn Fn(&mut State) -> anyhow::Result<()>>;

const CHUNK_SIZE: usize = 5;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DragMode {
    Char,
    Chunk,
    Line,
}

impl DragMode {
    fn from_click_count(count: usize) -> Self {
        match count {
            1 => Self::Chunk,
            2 => Self::Line,
            _ => Self::Char,
        }
    }
}

fn chunk_bounds(col: usize) -> (usize, usize) {
    let start = (col / CHUNK_SIZE) * CHUNK_SIZE;
    (start, start + CHUNK_SIZE)
}

pub struct MouseState {
    pub last_click: Option<Instant>,
    pub click_count: usize,

    pub click_handlers: [Option<ClickHandler>; 3],

    pub drag_anchor: Option<(LeafId, buffer::ID, usize, usize, DragMode)>,

    pub callback_sender: CallbackSender,
}

impl MouseState {
    pub fn new(sender: CallbackSender) -> Self {
        Self {
            last_click: None,
            click_count: 0,

            click_handlers: [const { None }; 3],
            drag_anchor: None,
            callback_sender: sender,
        }
    }

    /// The click-handler index the next click will dispatch to
    pub fn predicted_count(&self) -> usize {
        match self.last_click {
            Some(last_click) if Instant::now().duration_since(last_click).as_millis() >= 250 => 0,
            Some(_) => self.click_count,
            None => 0,
        }
    }

    pub fn handle_click(&mut self) {
        let time = Instant::now();

        if let Some(last_click) = self.last_click.as_ref()
            && Instant::now().duration_since(*last_click).as_millis() >= 250
        {
            self.click_count = 0;
        }

        let count = self.click_count;
        self.click_count += 1;

        if self.click_count == 3 {
            self.click_count = 0;
        }

        self.last_click = Some(time);

        self.callback_sender.send_fn(move |e| {
            let mut state = e.scope::<(MouseState,)>();
            let ((state,), e) = state.split();

            if let Some(handler) = &state.click_handlers[count] {
                handler(e)?;
            };

            Ok(())
        });
    }

    pub fn set_click_handler(
        &mut self,
        handler: usize,
        callback: impl Fn(&mut State) -> anyhow::Result<()> + 'static,
    ) {
        if handler >= self.click_handlers.len() {
            panic!("Can't set a click handler for a click that isn't tracked");
        }

        self.click_handlers[handler] = Some(Box::new(callback));
    }
}

pub fn init(editor: &mut Editor) {
    let sender = editor.sender();
    editor.insert(MouseState::new(sender));

    let mouse = editor.get::<&mut MouseState>();

    mouse.set_click_handler(0, |s| {
        let pos = *s.get::<&MousePosition>();
        let point = Position::new(pos.column, pos.row);

        let mut taken = s.scope::<(buffer::Storage, Pipeline, WindowLayout)>();
        let ((storage, pipeline, windows), state) = taken.split();

        if let Some((leaf, id, hit)) = windows.hit_test(storage, pipeline, state, point) {
            windows.focus(leaf);
            windows.active_mut().select(id.clone());

            if let Some(mut buf) = storage.get_mut(&id).and_then(|b| b.as_text()) {
                buf.set(hit.line, hit.col, &CursorOptions::new());
            }
        }

        Ok(())
    });

    mouse.set_click_handler(1, |s| {
        let pos = *s.get::<&MousePosition>();
        let point = Position::new(pos.column, pos.row);

        let mut taken = s.scope::<(buffer::Storage, Pipeline, WindowLayout)>();
        let ((storage, pipeline, windows), state) = taken.split();

        if let Some((leaf, id, hit)) = windows.hit_test(storage, pipeline, state, point) {
            windows.focus(leaf);
            windows.active_mut().select(id.clone());

            if let Some(mut buf) = storage.get_mut(&id).and_then(|b| b.as_text()) {
                //TODO: Make this select a word
                let (start, end) = chunk_bounds(hit.col);
                let end = (end - 1).min(buf.max_col(hit.line));

                buf.set(hit.line, start, &CursorOptions::new());
                buf.set(hit.line, end, &CursorOptions::new().extend(true));
            }
        }

        Ok(())
    });

    mouse.set_click_handler(2, |s| {
        let pos = *s.get::<&MousePosition>();
        let point = Position::new(pos.column, pos.row);

        let mut taken = s.scope::<(buffer::Storage, Pipeline, WindowLayout)>();
        let ((storage, pipeline, windows), state) = taken.split();

        if let Some((leaf, id, hit)) = windows.hit_test(storage, pipeline, state, point) {
            windows.focus(leaf);
            windows.active_mut().select(id.clone());

            if let Some(mut buf) = storage.get_mut(&id).and_then(|b| b.as_text()) {
                buf.set(hit.line, 0, &CursorOptions::new());
                buf.set(
                    hit.line,
                    buf.max_col(hit.line),
                    &CursorOptions::new().extend(true),
                );
            }
        }

        Ok(())
    });

    editor.events.on::<events::ExtraInput>(|s, e| {
        if Code::Mouse(MouseButton::Left) != e.chord.code {
            return Ok(());
        }

        s.get::<&mut MouseState>().handle_click();

        Ok(())
    });

    editor.events.on::<events::Input>(|s, e| {
        if Code::Mouse(MouseButton::Left) != e.chord.code {
            return Ok(());
        }

        let pos = *s.get::<&MousePosition>();
        let point = Position::new(pos.column, pos.row);
        let mode = DragMode::from_click_count(s.get::<&MouseState>().predicted_count());

        let mut taken = s.scope::<(buffer::Storage, Pipeline, WindowLayout)>();
        let ((storage, pipeline, windows), state) = taken.split();

        let anchor = windows
            .hit_test(storage, pipeline, state, point)
            .map(|(leaf, id, hit)| (leaf, id, hit.line, hit.col, mode));
        drop(taken);

        s.get::<&mut MouseState>().drag_anchor = anchor;

        Ok(())
    });

    editor.events.on::<events::MouseDrag>(|s, _| {
        let Some((leaf, id, start_line, start_col, mode)) =
            s.get::<&MouseState>().drag_anchor.clone()
        else {
            return Ok(());
        };

        let pos = *s.get::<&MousePosition>();
        let point = Position::new(pos.column, pos.row);

        let mut taken = s.scope::<(buffer::Storage, Pipeline, WindowLayout)>();
        let ((storage, pipeline, windows), state) = taken.split();

        if let Some(content_rect) = windows.window_mut(leaf).and_then(|w| w.content_rect()) {
            let total_lines = match storage.get(&id) {
                Some(buffer::Buffer::Text { buf, .. }) => buf.buffer.rope().len_lines(),
                _ => 0,
            };

            // Scroll faster the further the mouse has gone past the edge
            let delta = if point.y < content_rect.y {
                let overshoot = (content_rect.y - point.y) as isize;
                Some(-(1 + overshoot / 2))
            } else if point.y >= content_rect.y + content_rect.height {
                let overshoot = (point.y - (content_rect.y + content_rect.height - 1)) as isize;
                Some(1 + overshoot / 2)
            } else {
                None
            };

            if let Some(delta) = delta
                && let Some(window) = windows.window_mut(leaf)
            {
                window.scroll_by(delta, total_lines);
            }
        }

        if let Some((_, hit_id, hit)) = windows.hit_test(storage, pipeline, state, point)
            && hit_id == id
            && let Some(mut buf) = storage.get_mut(&id).and_then(|b| b.as_text())
        {
            match mode {
                DragMode::Char => {
                    buf.set(start_line, start_col, &CursorOptions::new());
                    buf.set(hit.line, hit.col, &CursorOptions::new().extend(true));
                }
                DragMode::Chunk => {
                    let (anchor_start, anchor_end) = chunk_bounds(start_col);
                    let (hit_start, hit_end) = chunk_bounds(hit.col);

                    if (hit.line, hit_start) >= (start_line, anchor_start) {
                        let end = (hit_end - 1).min(buf.max_col(hit.line));
                        buf.set(start_line, anchor_start, &CursorOptions::new());
                        buf.set(hit.line, end, &CursorOptions::new().extend(true));
                    } else {
                        let start = (anchor_end - 1).min(buf.max_col(start_line));
                        buf.set(start_line, start, &CursorOptions::new());
                        buf.set(hit.line, hit_start, &CursorOptions::new().extend(true));
                    }
                }
                DragMode::Line => {
                    if hit.line >= start_line {
                        buf.set(start_line, 0, &CursorOptions::new());
                        buf.set(
                            hit.line,
                            buf.max_col(hit.line),
                            &CursorOptions::new().extend(true),
                        );
                    } else {
                        buf.set(start_line, buf.max_col(start_line), &CursorOptions::new());
                        buf.set(hit.line, 0, &CursorOptions::new().extend(true));
                    }
                }
            }
        }

        Ok(())
    });

    editor.events.on::<events::MouseUp>(|s, _| {
        s.get::<&mut MouseState>().drag_anchor = None;

        Ok(())
    });
}
