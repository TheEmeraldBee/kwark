use std::sync::Arc;

use vello::util::{RenderContext, RenderSurface};
use vello::wgpu::util::TextureBlitterBuilder;
use vello::wgpu::{
    self, BlendComponent, BlendFactor, BlendOperation, BlendState, CompositeAlphaMode,
    CurrentSurfaceTexture,
};
use vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, Scene};
use winit::window::Window;

use crate::{Color, Error};

/// Copies straight alpha into a premultiplied surface
const PREMULTIPLY: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::Zero,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent::REPLACE,
};

/// The GPU device, shared by every window
pub(crate) struct Gpu {
    context: RenderContext,
    /// One renderer per device, indexed like `context.devices`
    renderers: Vec<Option<Renderer>>,
}

impl Gpu {
    pub fn new() -> Self {
        Self {
            context: RenderContext::new(),
            renderers: Vec::new(),
        }
    }

    /// Returns a surface for the window at its current size, see-through where alpha is below one if `transparent`
    pub fn surface(
        &mut self,
        window: Arc<Window>,
        transparent: bool,
    ) -> Result<RenderSurface<'static>, Error> {
        let size = window.inner_size();
        let mut surface = pollster::block_on(self.context.create_surface(
            window,
            size.width.max(1),
            size.height.max(1),
            wgpu::PresentMode::AutoVsync,
        ))?;

        let id = surface.dev_id;
        if self.renderers.len() <= id {
            self.renderers.resize_with(id + 1, || None);
        }

        if self.renderers[id].is_none() {
            let renderer = Renderer::new(
                &self.context.devices[id].device,
                RendererOptions {
                    antialiasing_support: AaSupport::area_only(),
                    ..RendererOptions::default()
                },
            )?;

            self.renderers[id] = Some(renderer);
        }

        if transparent {
            self.make_transparent(&mut surface);
        }

        Ok(surface)
    }

    /// Switches the surface to a mode where the compositor blends it with the desktop
    fn make_transparent(&self, surface: &mut RenderSurface<'static>) {
        let handle = &self.context.devices[surface.dev_id];
        let modes = surface
            .surface
            .get_capabilities(handle.adapter())
            .alpha_modes;

        // Vello writes straight alpha, so a premultiplied surface needs the copy to multiply it in
        let (mode, premultiply) = if modes.contains(&CompositeAlphaMode::PostMultiplied) {
            (CompositeAlphaMode::PostMultiplied, false)
        } else if modes.contains(&CompositeAlphaMode::PreMultiplied) {
            (CompositeAlphaMode::PreMultiplied, true)
        } else {
            eprintln!("kui: the surface supports no alpha blending, so the window stays opaque");
            return;
        };

        if premultiply {
            surface.blitter = TextureBlitterBuilder::new(&handle.device, surface.format)
                .blend_state(PREMULTIPLY)
                .build();
        }

        surface.config.alpha_mode = mode;
        self.context.configure_surface(surface);
    }

    /// Resizes the surface to a physical size, ignoring empty sizes
    pub fn resize(&self, surface: &mut RenderSurface<'static>, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.context.resize_surface(surface, width, height);
        }
    }

    /// Draws the scene over the background and shows it
    pub fn present(
        &mut self,
        surface: &RenderSurface<'static>,
        scene: &Scene,
        background: Color,
    ) -> Result<(), Error> {
        let frame = match surface.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => {
                frame
            }
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.context.configure_surface(surface);
                return Ok(());
            }
            // Nothing is visible, so the frame is skipped
            CurrentSurfaceTexture::Timeout
            | CurrentSurfaceTexture::Occluded
            | CurrentSurfaceTexture::Validation => return Ok(()),
        };

        let handle = &self.context.devices[surface.dev_id];
        let renderer = self.renderers[surface.dev_id]
            .as_mut()
            .expect("a renderer exists for each surface device");

        renderer.render_to_texture(
            &handle.device,
            &handle.queue,
            scene,
            &surface.target_view,
            &RenderParams {
                base_color: background.into(),
                width: surface.config.width,
                height: surface.config.height,
                antialiasing_method: AaConfig::Area,
            },
        )?;

        let mut encoder = handle
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("kui present"),
            });
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        surface
            .blitter
            .copy(&handle.device, &mut encoder, &surface.target_view, &view);

        handle.queue.submit([encoder.finish()]);
        handle.queue.present(frame);

        Ok(())
    }
}
