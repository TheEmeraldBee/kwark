use std::time::Instant;

use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::{Color, CursorIcon, Event, Fonts, Node, Rect, Size};

/// Options for the window that kui opens
#[derive(Clone, Debug)]
pub struct WindowOptions {
    pub title: String,
    /// Inner size in logical pixels
    pub size: Size,
    /// Color under everything the app draws
    pub background: Color,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "kui".into(),
            size: Size::new(1024.0, 768.0),
            background: Color::BLACK,
        }
    }
}

/// Access to the window from app code
pub struct Cx<'a> {
    pub(crate) fonts: &'a mut Fonts,
    pub(crate) window: &'a Window,
    pub(crate) size: Size,
    pub(crate) scale: f32,
    pub(crate) redraw: bool,
    pub(crate) quit: bool,
    pub(crate) keep_open: bool,
    pub(crate) cursor: Option<CursorIcon>,
}

impl<'a> Cx<'a> {
    pub(crate) fn new(fonts: &'a mut Fonts, window: &'a Window) -> Self {
        let scale = window.scale_factor();
        let size = window.inner_size().to_logical::<f32>(scale);

        Self {
            fonts,
            window,
            size: Size::new(size.width, size.height),
            scale: scale as f32,
            redraw: false,
            quit: false,
            keep_open: false,
            cursor: None,
        }
    }

    /// Returns the inner size of the window in logical pixels
    pub fn size(&self) -> Size {
        self.size
    }

    /// Returns the number of physical pixels in one logical pixel
    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn fonts(&mut self) -> &mut Fonts {
        self.fonts
    }

    /// Asks for a new frame
    pub fn request_redraw(&mut self) {
        self.redraw = true;
    }

    pub fn set_title(&mut self, title: &str) {
        self.window.set_title(title);
    }

    /// Sets the pointer icon for places where no node sets one
    pub fn set_cursor(&mut self, icon: CursorIcon) {
        self.cursor = Some(icon);
    }

    /// Tells the input method where the text cursor is, so that its popup sits beside it
    pub fn set_ime_area(&mut self, area: Rect) {
        self.window.set_ime_cursor_area(
            LogicalPosition::new(area.left(), area.top()),
            LogicalSize::new(area.size.width, area.size.height),
        );
    }

    /// Keeps the window open after `Event::CloseRequested`
    pub fn keep_open(&mut self) {
        self.keep_open = true;
    }

    /// Closes the app once the current event finishes
    pub fn quit(&mut self) {
        self.quit = true;
    }
}

/// A handle that wakes the app from any thread
pub struct Proxy<M: 'static>(pub(crate) EventLoopProxy<M>);

impl<M: 'static> Clone for Proxy<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<M: 'static> Proxy<M> {
    /// Sends a message to `App::message`, returning it if the app closed
    pub fn send(&self, message: M) -> Result<(), M> {
        self.0.send_event(message).map_err(|e| e.0)
    }
}

/// An application that kui drives
pub trait App: 'static {
    /// Data that element handlers receive
    type State: 'static;

    /// Messages from other threads
    type Message: Send + 'static;

    /// Options for the window
    fn window(&self) -> WindowOptions {
        WindowOptions::default()
    }

    /// Sets up the app once the window exists
    fn init(&mut self, _proxy: Proxy<Self::Message>, _cx: &mut Cx) {}

    /// Returns the state that element handlers receive
    fn state(&mut self) -> &mut Self::State;

    /// Handles an input event that no element took
    fn event(&mut self, _event: Event, _cx: &mut Cx) {}

    /// Handles a message from another thread
    fn message(&mut self, _message: Self::Message, _cx: &mut Cx) {}

    /// Returns the next time the app wants to wake
    fn deadline(&self) -> Option<Instant> {
        None
    }

    /// Handles a deadline that passed
    fn wake(&mut self, _now: Instant, _cx: &mut Cx) {}

    /// Returns the element tree for one frame
    fn view(&mut self, cx: &mut Cx) -> Node<Self::State>;
}
