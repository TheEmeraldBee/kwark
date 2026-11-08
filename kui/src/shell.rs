use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use taffy::TaffyTree;
use vello::Scene;
use vello::util::RenderSurface;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Ime, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

use crate::gpu::Gpu;
use crate::node::{Frame, Segment};
use crate::{
    App, Color, CursorIcon, Cx, Decorations, Error, Event, Fonts, Key, KeyEvent, Mods, MouseButton,
    Painter, Point, Proxy, ScrollDelta, Size,
};

/// One open window and what it draws with
struct WindowState<S> {
    window: Arc<Window>,
    surface: RenderSurface<'static>,
    scene: Scene,
    background: Color,
    /// The pointer position in logical pixels, while it is over the window
    pointer: Option<Point>,
    mods: Mods,
    taffy: TaffyTree<usize>,
    /// The last frame, for input until the next one
    frame: Option<Frame<S>>,
    /// The topmost node under the pointer in the last frame
    hit: Option<usize>,
    /// The path to the node that took the last mouse down
    pressed: Option<Vec<Segment>>,
    /// The pointer icon the app set for places where no node sets one
    app_cursor: CursorIcon,
    cursor: CursorIcon,
}

impl<S> WindowState<S> {
    fn scale(&self) -> f64 {
        self.window.scale_factor()
    }

    /// Returns the inner size in logical pixels
    fn size(&self) -> Size {
        let size = self.window.inner_size().to_logical::<f32>(self.scale());
        Size::new(size.width, size.height)
    }

    /// Shows the pointer icon of the hovered nodes, or the app's icon
    fn refresh_cursor(&mut self) {
        let icon = self
            .frame
            .as_ref()
            .and_then(|f| f.cursor(self.hit))
            .unwrap_or(self.app_cursor);

        if icon != self.cursor {
            self.window.set_cursor(icon);
            self.cursor = icon;
        }
    }

    /// Moves hover to the node under the pointer, asking for a frame if a look changes
    fn update_hover(&mut self, cx: &mut Cx) {
        let Some(frame) = &self.frame else {
            return;
        };

        let hit = self.pointer.and_then(|p| frame.hit(p));
        if hit != self.hit && (frame.hover_styled(self.hit) || frame.hover_styled(hit)) {
            cx.request_redraw();
        }

        self.hit = hit;
        self.refresh_cursor();
    }

    /// Sends a mouse event to the node it lands on, returning whether a node took it
    fn dispatch(&mut self, event: &Event, state: &mut S, cx: &mut Cx) -> bool {
        match *event {
            Event::MouseMove { .. } | Event::MouseLeave => {
                self.update_hover(cx);
                false
            }
            Event::MouseDown {
                position,
                button: MouseButton::Left,
            } => {
                let Some(frame) = &self.frame else {
                    return false;
                };

                let target = frame.hit(position).and_then(|h| frame.clickable(h));
                self.pressed = target.map(|t| frame.path(t));
                target.is_some()
            }
            Event::MouseUp {
                position,
                button: MouseButton::Left,
            } => {
                let (Some(frame), Some(pressed)) = (&mut self.frame, self.pressed.take()) else {
                    return false;
                };

                let target = frame.hit(position).and_then(|h| frame.clickable(h));
                if let Some(t) = target
                    && frame.path(t) == pressed
                    && let Some(on_click) = frame.nodes[t].on_click.as_mut()
                {
                    on_click(state, cx);
                }

                true
            }
            _ => false,
        }
    }
}

/// The winit handler that drives an app
struct Shell<A: App> {
    app: A,
    gpu: Gpu,
    fonts: Fonts,
    proxy: EventLoopProxy<A::Message>,
    windows: HashMap<WindowId, WindowState<A::State>>,
    error: Option<Error>,
}

impl<A: App> Shell<A> {
    fn new(app: A, proxy: EventLoopProxy<A::Message>) -> Self {
        Self {
            app,
            gpu: Gpu::new(),
            fonts: Fonts::new(),
            proxy,
            windows: HashMap::new(),
            error: None,
        }
    }

    /// Opens the app's window and adds it to the window map
    fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<WindowId, Error> {
        let options = self.app.window();
        let attributes = Window::default_attributes()
            .with_title(options.title)
            .with_inner_size(LogicalSize::new(options.size.width, options.size.height))
            .with_resizable(options.resizable)
            .with_transparent(options.transparent)
            .with_blur(options.blur)
            .with_decorations(options.decorations != Decorations::None);

        #[cfg(target_os = "macos")]
        let attributes = {
            use winit::platform::macos::WindowAttributesExtMacOS;

            let overlay = options.decorations == Decorations::Overlay;
            attributes
                .with_titlebar_transparent(overlay)
                .with_title_hidden(overlay)
                .with_fullsize_content_view(overlay)
        };

        #[cfg(target_os = "windows")]
        let attributes = {
            use winit::platform::windows::WindowAttributesExtWindows;

            attributes.with_undecorated_shadow(options.decorations == Decorations::None)
        };

        let window = Arc::new(event_loop.create_window(attributes)?);
        window.set_ime_allowed(true);

        let surface = self.gpu.surface(window.clone(), options.transparent)?;
        let id = window.id();

        self.windows.insert(
            id,
            WindowState {
                window,
                surface,
                scene: Scene::new(),
                background: options.background,
                pointer: None,
                mods: Mods::default(),
                taffy: TaffyTree::new(),
                frame: None,
                hit: None,
                pressed: None,
                app_cursor: CursorIcon::Default,
                cursor: CursorIcon::Default,
            },
        );

        Ok(id)
    }

    /// Stops the loop and keeps the error for `run` to return
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: Error) {
        self.error = Some(error);
        event_loop.exit();
    }

    /// Returns the first open window
    fn main_window(&self) -> Option<WindowId> {
        self.windows.keys().next().copied()
    }

    /// Runs app code with a `Cx` for a window, then applies what the code asked for
    fn with_cx<R>(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        f: impl FnOnce(&mut A, &mut WindowState<A::State>, &mut Cx) -> R,
    ) -> Option<R> {
        let state = self.windows.get_mut(&id)?;
        let window = state.window.clone();
        let mut cx = Cx::new(&mut self.fonts, &window, state.mods);

        let result = f(&mut self.app, state, &mut cx);

        if let Some(icon) = cx.cursor {
            state.app_cursor = icon;
            state.refresh_cursor();
        }
        if cx.redraw {
            window.request_redraw();
        }
        if cx.quit {
            event_loop.exit();
        }

        Some(result)
    }

    /// Gives an event to the node it lands on, or else to the app
    fn send(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: Event) {
        self.with_cx(event_loop, id, |app, state, cx| {
            if !state.dispatch(&event, app.state(), cx) {
                app.event(event, cx);
            }
        });
    }

    /// Builds, lays out, paints, and shows one frame
    fn redraw(&mut self, event_loop: &ActiveEventLoop, id: WindowId) {
        let Some(root) = self.with_cx(event_loop, id, |app, _, cx| app.view(cx)) else {
            return;
        };
        let Some(state) = self.windows.get_mut(&id) else {
            return;
        };

        let size = state.size();
        let scale = state.scale();

        let mut frame = match Frame::build(root, size, &mut self.fonts, &mut state.taffy) {
            Ok(frame) => frame,
            Err(e) => return self.fail(event_loop, e),
        };

        state.hit = state.pointer.and_then(|p| frame.hit(p));

        state.scene.reset();
        let mut painter = Painter::new(
            &mut state.scene,
            &mut self.fonts,
            scale,
            size,
            state.pointer,
        );
        frame.paint(&mut painter, self.app.state(), state.hit);
        state.frame = Some(frame);
        state.refresh_cursor();
        self.fonts.end_frame();

        let res = self
            .gpu
            .present(&state.surface, &state.scene, state.background);

        if let Err(e) = res {
            self.fail(event_loop, e);
        }
    }
}

/// Returns the kui event for a winit event, updating the window's input state
fn translate<S>(state: &mut WindowState<S>, event: &WindowEvent) -> Option<Event> {
    let scale = state.scale();

    match event {
        WindowEvent::CursorMoved { position, .. } => {
            let position = position.to_logical::<f32>(scale);
            let position = Point::new(position.x, position.y);
            state.pointer = Some(position);

            Some(Event::MouseMove { position })
        }
        WindowEvent::CursorLeft { .. } => {
            state.pointer = None;
            Some(Event::MouseLeave)
        }
        WindowEvent::MouseInput {
            state: pressed,
            button,
            ..
        } => {
            let position = state.pointer.unwrap_or_default();
            let button = (*button).into();

            Some(match pressed {
                ElementState::Pressed => Event::MouseDown { position, button },
                ElementState::Released => Event::MouseUp { position, button },
            })
        }
        WindowEvent::MouseWheel { delta, .. } => {
            let delta = match *delta {
                MouseScrollDelta::LineDelta(x, y) => ScrollDelta::Lines(Point::new(x, y)),
                MouseScrollDelta::PixelDelta(p) => {
                    let p = p.to_logical::<f32>(scale);
                    ScrollDelta::Pixels(Point::new(p.x, p.y))
                }
            };

            Some(Event::Scroll {
                position: state.pointer.unwrap_or_default(),
                delta,
            })
        }
        WindowEvent::ModifiersChanged(mods) => {
            state.mods = mods.state().into();
            None
        }
        WindowEvent::KeyboardInput { event, .. } => {
            if event.state != ElementState::Pressed {
                return None;
            }

            let key = Key::from_winit(&event.logical_key)?;
            let mods = state.mods;
            let text = event
                .text
                .as_ref()
                .filter(|t| !mods.ctrl && !mods.logo && !t.chars().any(char::is_control))
                .map(|t| t.to_string());

            Some(Event::Key(KeyEvent {
                key,
                text,
                mods,
                repeat: event.repeat,
            }))
        }
        WindowEvent::Ime(Ime::Commit(text)) => Some(Event::Text(text.clone())),
        WindowEvent::Ime(Ime::Preedit(text, cursor)) => Some(Event::Preedit {
            text: text.clone(),
            cursor: *cursor,
        }),
        WindowEvent::Resized(_) => Some(Event::Resize(state.size())),
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            Some(Event::Scale(*scale_factor as f32))
        }
        WindowEvent::Focused(focused) => Some(Event::Focus(*focused)),
        _ => None,
    }
}

impl<A: App> ApplicationHandler<A::Message> for Shell<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }

        let id = match self.open(event_loop) {
            Ok(id) => id,
            Err(e) => return self.fail(event_loop, e),
        };

        let proxy = Proxy(self.proxy.clone());
        self.with_cx(event_loop, id, |app, _, cx| app.init(proxy, cx));
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, message: A::Message) {
        if let Some(id) = self.main_window() {
            self.with_cx(event_loop, id, |app, _, cx| app.message(message, cx));
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(state) = self.windows.get_mut(&id) else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                let keep = self.with_cx(event_loop, id, |app, _, cx| {
                    app.event(Event::CloseRequested, cx);
                    cx.keep_open
                });

                if keep != Some(true) {
                    self.windows.remove(&id);
                }
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop, id),
            _ => {
                match event {
                    WindowEvent::Resized(size) => {
                        self.gpu.resize(&mut state.surface, size.width, size.height);
                        state.window.request_redraw();
                    }
                    WindowEvent::ScaleFactorChanged { .. } => state.window.request_redraw(),
                    _ => {}
                }

                if let Some(event) = translate(state, &event) {
                    self.send(event_loop, id, event);
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(id) = self.main_window() else {
            return;
        };

        let now = Instant::now();
        if self.app.deadline().is_some_and(|t| t <= now) {
            self.with_cx(event_loop, id, |app, _, cx| app.wake(now, cx));
        }

        event_loop.set_control_flow(match self.app.deadline() {
            Some(t) => ControlFlow::WaitUntil(t),
            None => ControlFlow::Wait,
        });
    }
}

/// Runs the app until it quits or its last window closes
pub fn run<A: App>(app: A) -> Result<(), Error> {
    let event_loop = EventLoop::<A::Message>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut shell = Shell::new(app, event_loop.create_proxy());
    event_loop.run_app(&mut shell)?;

    match shell.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
