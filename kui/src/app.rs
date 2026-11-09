use std::time::Instant;

use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::{EventLoopClosed, EventLoopProxy};
pub use winit::window::ResizeDirection;
use winit::window::Window;

use crate::{Color, CursorIcon, Event, Fonts, Mods, Node, Point, Rect, Size};

/// Options to change how the window is handled
#[derive(Clone, Debug)]
pub struct WindowOptions {
    /// The title of the window
    pub title: String,
    /// The size of the window
    pub size: Size,
    /// The background color of the window
    pub background: Color,
    /// The decorations mode that this will use (title-bar)
    pub decorations: Decorations,
    /// Whether the window is resizable
    pub resizable: bool,
    /// Whether the window needs to support transparency or not
    pub transparent: bool,
    /// Whether or not to blur the window (transparency must be true)
    pub blur: bool,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "kui".into(),
            size: Size::new(1024.0, 768.0),
            background: Color::BLACK,
            decorations: Decorations::default(),
            resizable: true,
            transparent: false,
            blur: false,
        }
    }
}

/// The way window decorations are rendered
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Decorations {
    /// Take the one from the system
    #[default]
    System,
    /// Don't render native system decorations
    None,
    /// Try to use the transparent title bar on macOS, default to the system frame
    Overlay,
}

/// The window context
pub struct Cx<'a> {
    pub(crate) fonts: &'a mut Fonts,
    pub(crate) window: &'a Window,
    pub(crate) size: Size,
    pub(crate) scale: f32,
    pub(crate) mods: Mods,
    pub(crate) redraw: bool,
    pub(crate) quit: bool,
    pub(crate) keep_open: bool,
    pub(crate) cursor: Option<CursorIcon>,
}

impl<'a> Cx<'a> {
    pub(crate) fn new(fonts: &'a mut Fonts, window: &'a Window, mods: Mods) -> Self {
        let scale = window.scale_factor();
        let size = window.inner_size().to_logical::<f32>(scale);

        Self {
            fonts,
            window,
            size: Size::new(size.width, size.height),
            scale: scale as f32,
            mods,
            redraw: false,
            quit: false,
            keep_open: false,
            cursor: None,
        }
    }

    /// Returns the inner logical size of the window
    pub fn size(&self) -> Size {
        self.size
    }

    /// Returns the number of physical pixels in one logical pixel
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Returns the modifier keys currently pressed
    pub fn mods(&self) -> Mods {
        self.mods
    }

    /// Returns the fonts data
    pub fn fonts(&mut self) -> &mut Fonts {
        self.fonts
    }

    /// Tell the window to redraw
    pub fn request_redraw(&mut self) {
        self.redraw = true;
    }

    /// Set the window title
    pub fn set_title(&mut self, title: &str) {
        self.window.set_title(title);
    }

    /// Sets the default cursor icon
    pub fn set_cursor(&mut self, icon: CursorIcon) {
        self.cursor = Some(icon);
    }

    /// Tells the input where the text cursor is for IME to work
    pub fn set_ime_area(&mut self, area: Rect) {
        self.window.set_ime_cursor_area(
            LogicalPosition::new(area.left(), area.top()),
            LogicalSize::new(area.size.width, area.size.height),
        );
    }

    /// Moves the window with the pointer
    pub fn drag_window(&mut self) {
        let _ = self.window.drag_window();
    }

    /// Resizes the window with the pointer
    pub fn drag_resize(&mut self, direction: ResizeDirection) {
        let _ = self.window.drag_resize_window(direction);
    }

    /// Opens the system menu for the window at a point
    pub fn show_window_menu(&mut self, at: Point) {
        self.window
            .show_window_menu(LogicalPosition::new(at.x, at.y));
    }

    /// Shows or hides the system frame
    pub fn set_decorations(&mut self, decorations: bool) {
        self.window.set_decorations(decorations);
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
    /// Sends a message to `App::message`, returns a winit error if it fails
    pub fn send(&self, message: M) -> Result<(), EventLoopClosed<M>> {
        self.0.send_event(message)
    }
}

/// An application for kui
pub trait App: 'static {
    /// The State that the App handles
    type State: 'static;

    /// Messages from other threads
    type Message: Send + 'static;

    /// Options to apply to the window
    fn window(&self) -> WindowOptions {
        WindowOptions::default()
    }

    /// Sets up the app
    fn init(&mut self, _proxy: Proxy<Self::Message>, _cx: &mut Cx) {}

    /// Returns the state the App needs
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
