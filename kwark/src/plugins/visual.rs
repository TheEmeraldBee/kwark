use std::time::Instant;

use crossterm::event::MouseButton;
use kwark_text_buffer::CursorOptions;
use ratatui::layout::Position;

use crate::{CallbackSender, events, prelude::*};

type ClickHandler = Box<dyn Fn(&mut State) -> anyhow::Result<()>>;

pub struct MouseState {
    pub last_click: Option<Instant>,
    pub click_count: usize,

    pub click_handlers: [Option<ClickHandler>; 3],

    pub callback_sender: CallbackSender,
}

impl MouseState {
    pub fn new(sender: CallbackSender) -> Self {
        Self {
            last_click: None,
            click_count: 0,

            click_handlers: [const { None }; 3],
            callback_sender: sender,
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
                buf.set(hit.line, hit.col.saturating_sub(3), &CursorOptions::new());
                buf.set(
                    hit.line,
                    hit.col.saturating_add(3),
                    &CursorOptions::new().extend(true),
                );
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
}
