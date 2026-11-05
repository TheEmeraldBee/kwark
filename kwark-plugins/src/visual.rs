use std::time::Instant;

use crossterm::event::MouseButton;
use kwark_text::CursorOptions;
use ratatui::layout::Position;

use kwark_core::{events, prelude::*};

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

/// The click and drag state of the mouse
#[derive(Default)]
pub struct MouseState {
    pub last_click: Option<Instant>,
    pub click_count: usize,

    pub drag_anchor: Option<(LeafId, buffer::ID, usize, usize, DragMode)>,
}

impl MouseState {
    /// The click-handler index the next click will dispatch to
    pub fn predicted_count(&self) -> usize {
        match self.last_click {
            Some(last_click) if Instant::now().duration_since(last_click).as_millis() >= 250 => 0,
            Some(_) => self.click_count,
            None => 0,
        }
    }

    /// Records a click, returning its index in a multi-click
    pub fn handle_click(&mut self) -> usize {
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

        count
    }
}

/// Focuses the clicked window and buffer, returning the buffer and the text position
fn focus_click<S: 'static>(s: &mut Editor<S>) -> Option<(buffer::ID, buffer::HitPosition)> {
    let pos = s.core.mouse;
    let point = Position::new(pos.column, pos.row);

    let (leaf, id, hit) = s.hit_test(point)?;

    s.core.windows.focus(leaf);
    s.core.windows.active_mut().select(id.clone());

    Some((id, hit))
}

fn click_char<S: 'static>(s: &mut Editor<S>) {
    let Some((id, hit)) = focus_click(s) else {
        return;
    };

    if let Some(mut buf) = s.core.buffers.get_mut(&id).and_then(|b| b.as_text()) {
        buf.set(hit.line, hit.col, &CursorOptions::new());
    }
}

fn click_chunk<S: 'static>(s: &mut Editor<S>) {
    let Some((id, hit)) = focus_click(s) else {
        return;
    };

    if let Some(mut buf) = s.core.buffers.get_mut(&id).and_then(|b| b.as_text()) {
        //TODO: Make this select a word
        let (start, end) = chunk_bounds(hit.col);
        let end = (end - 1).min(buf.max_col(hit.line));

        buf.set(hit.line, start, &CursorOptions::new());
        buf.set(hit.line, end, &CursorOptions::new().extend(true));
    }
}

fn click_line<S: 'static>(s: &mut Editor<S>) {
    let Some((id, hit)) = focus_click(s) else {
        return;
    };

    if let Some(mut buf) = s.core.buffers.get_mut(&id).and_then(|b| b.as_text()) {
        buf.set(hit.line, 0, &CursorOptions::new());
        buf.set(
            hit.line,
            buf.max_col(hit.line),
            &CursorOptions::new().extend(true),
        );
    }
}

/// Click, multi-click, and drag selection with the mouse
#[derive(Default)]
pub struct Plugin;

impl PluginState for Plugin {
    type State = MouseState;

    fn init(&self) -> anyhow::Result<MouseState> {
        Ok(MouseState::default())
    }
}

impl<S: Has<Plugin> + 'static> kwark_core::Plugin<S> for Plugin {
    fn register(&self, editor: &mut Editor<S>) -> anyhow::Result<()> {
        register(editor);
        Ok(())
    }
}

fn register<S: Has<Plugin> + 'static>(editor: &mut Editor<S>) {
    editor.events.on::<events::ExtraInput>(|s, e| {
        if Code::Mouse(MouseButton::Left) != e.chord.code {
            return Ok(());
        }

        match s.plugin_mut::<Plugin>().handle_click() {
            0 => click_char(s),
            1 => click_chunk(s),
            _ => click_line(s),
        }

        Ok(())
    });

    editor.events.on::<events::Input>(|s, e| {
        if Code::Mouse(MouseButton::Left) != e.chord.code {
            return Ok(());
        }

        let pos = s.core.mouse;
        let point = Position::new(pos.column, pos.row);
        let mode = DragMode::from_click_count(s.plugin::<Plugin>().predicted_count());

        let anchor = s
            .hit_test(point)
            .map(|(leaf, id, hit)| (leaf, id, hit.line, hit.col, mode));

        s.plugin_mut::<Plugin>().drag_anchor = anchor;

        Ok(())
    });

    editor.events.on::<events::MouseDrag>(|s, _| {
        let Some((leaf, id, start_line, start_col, mode)) =
            s.plugin::<Plugin>().drag_anchor.clone()
        else {
            return Ok(());
        };

        let pos = s.core.mouse;
        let point = Position::new(pos.column, pos.row);

        s.with_view(|v, state| {
            let View {
                buffers: storage,
                windows,
                pipeline,
                ..
            } = v;

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
        });

        Ok(())
    });

    editor.events.on::<events::MouseUp>(|s, _| {
        s.plugin_mut::<Plugin>().drag_anchor = None;

        Ok(())
    });
}
