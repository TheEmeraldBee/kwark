use std::collections::HashMap;
use std::sync::Arc;

use vello::Scene;
use vello::util::RenderSurface;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::gpu::Gpu;
use crate::{App, Color, Error};

/// One open window and what it draws with
struct WindowState {
    window: Arc<Window>,
    surface: RenderSurface<'static>,
    scene: Scene,
    background: Color,
}

/// The winit handler that drives an app
struct Shell<A: App> {
    app: A,
    gpu: Gpu,
    windows: HashMap<WindowId, WindowState>,
    error: Option<Error>,
}

impl<A: App> Shell<A> {
    fn new(app: A) -> Self {
        Self {
            app,
            gpu: Gpu::new(),
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
            WindowEvent::RedrawRequested => {
                state.scene.reset();

                let res = self
                    .gpu
                    .present(&state.surface, &state.scene, state.background);

                if let Err(e) = res {
                    self.fail(event_loop, e);
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
