use std::collections::HashMap;
use std::sync::Arc;

use taffy::TaffyTree;
use vello::Scene;
use vello::util::RenderSurface;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::gpu::Gpu;
use crate::node::{Frame, Segment};
use crate::{
    App, Color, CursorIcon, Cx, Error, Event, Fonts, MouseButton, Painter, Point, ScrollDelta, Size,
};

/// One open window and what it draws with
struct WindowState<S> {
    window: Arc<Window>,
    surface: RenderSurface<'static>,
    scene: Scene,
    background: Color,
    /// The pointer position in logical pixels, while it is over the window
    pointer: Option<Point>,
    taffy: TaffyTree<usize>,
    /// The last frame, for input until the next one
    frame: Option<Frame<S>>,
    /// The topmost node under the pointer in the last frame
    hit: Option<usize>,
    /// The path to the node that took the last mouse down
    pressed: Option<Vec<Segment>>,
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

        let cursor = frame.cursor(hit);
        if cursor != self.cursor {
            self.window.set_cursor(cursor);
            self.cursor = cursor;
        }
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
    windows: HashMap<WindowId, WindowState<A::State>>,
    error: Option<Error>,
}

impl<A: App> Shell<A> {
    fn new(app: A) -> Self {
        Self {
            app,
            gpu: Gpu::new(),
            fonts: Fonts::new(),
            windows: HashMap::new(),
            error: None,
        }
    }

    /// Opens the app's window and adds it to the window map
    fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Error> {
        let options = self.app.window();
        let attributes = Window::default_attributes()
            .with_title(options.title)
            .with_inner_size(LogicalSize::new(options.size.width, options.size.height));

        let window = Arc::new(event_loop.create_window(attributes)?);
        let surface = self.gpu.surface(window.clone())?;

        self.windows.insert(
            window.id(),
            WindowState {
                window,
                surface,
                scene: Scene::new(),
                background: options.background,
                pointer: None,
                taffy: TaffyTree::new(),
                frame: None,
                hit: None,
                pressed: None,
                cursor: CursorIcon::Default,
            },
        );

        Ok(())
    }

    /// Stops the loop and keeps the error for `run` to return
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: Error) {
        self.error = Some(error);
        event_loop.exit();
    }
}

/// Returns the kui event for a winit event, updating the window's pointer
fn translate<S>(state: &mut WindowState<S>, event: &WindowEvent) -> Option<Event> {
    let scale = state.scale();

    match *event {
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
            let button = button.into();

            Some(match pressed {
                ElementState::Pressed => Event::MouseDown { position, button },
                ElementState::Released => Event::MouseUp { position, button },
            })
        }
        WindowEvent::MouseWheel { delta, .. } => {
            let delta = match delta {
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
        _ => None,
    }
}

impl<A: App> ApplicationHandler for Shell<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }

        if let Err(e) = self.open(event_loop) {
            self.fail(event_loop, e);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(state) = self.windows.get_mut(&id) else {
            return;
        };

        if let Some(event) = translate(state, &event) {
            let mut cx = Cx {
                fonts: &mut self.fonts,
                size: state.size(),
                scale: state.scale() as f32,
                redraw: false,
            };

            if !state.dispatch(&event, self.app.state(), &mut cx) {
                self.app.event(event, &mut cx);
            }

            if cx.redraw {
                state.window.request_redraw();
            }

            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.windows.remove(&id);

                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                self.gpu.resize(&mut state.surface, size.width, size.height);
                state.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let size = state.size();
                let scale = state.scale();

                let mut cx = Cx {
                    fonts: &mut self.fonts,
                    size,
                    scale: scale as f32,
                    redraw: false,
                };
                let root = self.app.view(&mut cx);
                let again = cx.redraw;

                let mut frame = match Frame::build(root, size, &mut self.fonts, &mut state.taffy) {
                    Ok(frame) => frame,
                    Err(e) => return self.fail(event_loop, e),
                };

                state.hit = state.pointer.and_then(|p| frame.hit(p));
                let cursor = frame.cursor(state.hit);
                if cursor != state.cursor {
                    state.window.set_cursor(cursor);
                    state.cursor = cursor;
                }

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
                self.fonts.end_frame();

                let res = self
                    .gpu
                    .present(&state.surface, &state.scene, state.background);

                if let Err(e) = res {
                    return self.fail(event_loop, e);
                }

                if again {
                    state.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

/// Runs the app until its last window closes
pub fn run<A: App>(app: A) -> Result<(), Error> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut shell = Shell::new(app);
    event_loop.run_app(&mut shell)?;

    match shell.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
