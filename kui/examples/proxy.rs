//! A count from a background thread, a blinking caret, and typed text

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use kui::{
    App, Color, Cx, Event, Family, Key, KeyEvent, NamedKey, Node, Proxy, Rect, Size, TextStyle,
    WindowOptions, div, text,
};

const BG: Color = Color::rgb(0x1E1E2E);
const PANEL: Color = Color::rgb(0x181825);
const BORDER: Color = Color::rgb(0x45475A);
const FG: Color = Color::rgb(0xCDD6F4);
const MUTED: Color = Color::rgb(0x7F849C);
const CARET: Color = Color::rgb(0xF5E0DC);
const ACCENT: Color = Color::rgb(0x89B4FA);

const BLINK: Duration = Duration::from_millis(530);
/// How long the caret blinks after the last input
const BLINK_FOR: Duration = Duration::from_secs(10);

const PAD: f32 = 32.0;
const INPUT_PX: f32 = 12.0;
const INPUT_PY: f32 = 8.0;
const FONT_SIZE: f32 = 16.0;

enum Message {
    Tick(u64),
}

struct Demo {
    count: u64,
    paused: Arc<AtomicBool>,
    input: String,
    preedit: String,
    caret_on: bool,
    next_blink: Instant,
    last_input: Instant,
    focused: bool,
    frames: u64,
    confirm_close: bool,
}

impl Demo {
    fn new() -> Self {
        let now = Instant::now();

        Self {
            count: 0,
            paused: Arc::new(AtomicBool::new(false)),
            input: String::new(),
            preedit: String::new(),
            caret_on: true,
            next_blink: now + BLINK,
            last_input: now,
            focused: true,
            frames: 0,
            confirm_close: false,
        }
    }

    /// Shows the caret and restarts the blink after input
    fn touch(&mut self, cx: &mut Cx) {
        let now = Instant::now();
        self.caret_on = true;
        self.next_blink = now + BLINK;
        self.last_input = now;
        self.confirm_close = false;
        cx.request_redraw();
    }

    fn blinking(&self) -> bool {
        self.focused && self.last_input.elapsed() < BLINK_FOR
    }

    fn key(&mut self, key: KeyEvent, cx: &mut Cx) {
        match key.key {
            Key::Named(NamedKey::Escape) => cx.quit(),
            Key::Named(NamedKey::Backspace) => {
                self.input.pop();
            }
            Key::Named(NamedKey::Enter) => cx.set_title(&format!("kui proxy: {}", self.input)),
            Key::Named(NamedKey::Tab) => {
                let paused = !self.paused.load(Ordering::Relaxed);
                self.paused.store(paused, Ordering::Relaxed);
            }
            _ => {
                if let Some(text) = key.text {
                    self.input.push_str(&text);
                }
            }
        }

        self.touch(cx);
    }

    fn line(label: &str, value: String) -> Node<Self> {
        div()
            .flex_row()
            .gap(8.0)
            .child(text(label).color(MUTED).w(90.0))
            .child(text(value))
    }
}

impl App for Demo {
    type State = Self;
    type Message = Message;

    fn window(&self) -> WindowOptions {
        WindowOptions {
            title: "kui proxy".into(),
            size: Size::new(720.0, 420.0),
            background: BG,
            ..WindowOptions::default()
        }
    }

    fn init(&mut self, proxy: Proxy<Message>, _cx: &mut Cx) {
        let paused = self.paused.clone();

        thread::spawn(move || {
            let mut n = 0;
            loop {
                thread::sleep(Duration::from_secs(1));
                if paused.load(Ordering::Relaxed) {
                    continue;
                }

                n += 1;
                if proxy.send(Message::Tick(n)).is_err() {
                    break;
                }
            }
        });
    }

    fn state(&mut self) -> &mut Self {
        self
    }

    fn event(&mut self, event: Event, cx: &mut Cx) {
        match event {
            Event::Key(key) => self.key(key, cx),
            Event::Text(text) => {
                self.input.push_str(&text);
                self.preedit.clear();
                self.touch(cx);
            }
            Event::Preedit { text, .. } => {
                self.preedit = text;
                self.touch(cx);
            }
            Event::Focus(focused) => {
                self.focused = focused;
                self.touch(cx);
            }
            Event::CloseRequested if !self.input.is_empty() && !self.confirm_close => {
                self.confirm_close = true;
                cx.keep_open();
                cx.request_redraw();
            }
            _ => {}
        }
    }

    fn message(&mut self, message: Message, cx: &mut Cx) {
        match message {
            Message::Tick(n) => self.count = n,
        }
        cx.request_redraw();
    }

    fn deadline(&self) -> Option<Instant> {
        // One more wake after the blink stops, so that the caret ends up shown
        let wake = self.focused && (self.blinking() || !self.caret_on);
        wake.then_some(self.next_blink)
    }

    fn wake(&mut self, now: Instant, cx: &mut Cx) {
        self.caret_on = !self.caret_on || !self.blinking();
        self.next_blink = now + BLINK;
        cx.request_redraw();
    }

    fn view(&mut self, cx: &mut Cx) -> Node<Self> {
        self.frames += 1;

        let style = TextStyle::default()
            .family(Family::Monospace)
            .size(FONT_SIZE);
        let typed = format!("{}{}", self.input, self.preedit);
        let width = cx
            .fonts()
            .line(&[kui::Span::new(&typed, style.clone())])
            .width();
        let height = cx.fonts().line_height(&style);

        let x = PAD + 1.0 + INPUT_PX + width;
        let y = PAD + 1.0 + INPUT_PY;
        cx.set_ime_area(Rect::new(x, y, 2.0, height));

        let caret_visible = self.caret_on;
        let caret = div()
            .w(2.0)
            .h(height)
            .bg(CARET)
            .opacity(if caret_visible && self.focused {
                1.0
            } else {
                0.0
            });

        let input = div()
            .flex_row()
            .items_center()
            .px(INPUT_PX)
            .py(INPUT_PY)
            .rounded(6.0)
            .bg(PANEL)
            .border(1.0, if self.focused { ACCENT } else { BORDER })
            .font(Family::Monospace)
            .size(FONT_SIZE)
            .child(text(self.input.clone()))
            .child(text(self.preedit.clone()).underline().color(ACCENT))
            .child(caret);

        let paused = self.paused.load(Ordering::Relaxed);
        let hint = if self.confirm_close {
            "Close again to discard the text".to_string()
        } else {
            "Type, Enter sets the title, Tab pauses the count, Esc quits".to_string()
        };

        div()
            .flex_col()
            .gap(14.0)
            .p(PAD)
            .font(Family::SansSerif)
            .size(14.0)
            .color(FG)
            .child(input)
            .child(Self::line(
                "count",
                format!("{}{}", self.count, if paused { " (paused)" } else { "" }),
            ))
            .child(Self::line("frames", self.frames.to_string()))
            .child(Self::line(
                "caret",
                if self.blinking() { "blinking" } else { "idle" }.to_string(),
            ))
            .child(text(hint).color(MUTED))
    }
}

fn main() {
    if let Err(e) = kui::run(Demo::new()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
