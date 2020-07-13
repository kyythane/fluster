#![deny(clippy::all)]
use fluster_core::{
    engine::Engine,
    rendering::{lin_srgb_to_coloru, paint, Renderer as FlusterRenderer},
};
use fluster_graphics::FlusterRendererImpl;
use gl::{ReadPixels, BGRA, UNSIGNED_BYTE};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
    ContextBuilder, ContextWrapper, GlProfile, GlRequest, PossiblyCurrent,
};
use iced::image::Handle as ImageHandle;
use palette::LinSrgb;
use pathfinder_canvas::CanvasFontContext;
use pathfinder_color::ColorF;
use pathfinder_geometry::vector::Vector2I;
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererMode, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_resources::embedded::EmbeddedResourceLoader;
use std::{convert::TryInto, error::Error, ffi::c_void};

//TODO: This class is temporary until Iced has smoother integration

pub struct StageRenderer {
    renderer: FlusterRendererImpl<GLDevice>,
    stage_size: Vector2I,
    gl_context: ContextWrapper<PossiblyCurrent, Window>,
}

impl StageRenderer {
    pub fn new(stage_size: Vector2I) -> Result<StageRenderer, String> {
        let event_loop = EventLoop::new();
        let physical_window_size = PhysicalSize::new(stage_size.x() as f64, stage_size.y() as f64);

        let window_builder = WindowBuilder::new()
            .with_title("Fluster Temp Renderer")
            .with_inner_size(physical_window_size);

        let gl_context = ContextBuilder::new()
            .with_gl(GlRequest::Latest)
            .with_gl_profile(GlProfile::Core)
            .build_windowed(window_builder, &event_loop)
            .unwrap();

        let gl_context = unsafe { gl_context.make_current().unwrap() };
        gl::load_with(|name| gl_context.get_proc_address(name) as *const _);

        gl_context.window().set_visible(false);

        let device = GLDevice::new(GLVersion::GL3, 0);
        let renderer_mode = RendererMode::default_for_device(&device);
        let renderer = Renderer::new(
            device,
            &EmbeddedResourceLoader,
            renderer_mode,
            RendererOptions {
                dest: DestFramebuffer::full_window(stage_size),
                background_color: Some(ColorF::white()),
                show_debug_ui: false,
            },
        );

        let font_context = CanvasFontContext::from_system_source();

        let fluster_renderer = FlusterRendererImpl::new(font_context, renderer, Box::new(|| ()));
        Ok(StageRenderer {
            renderer: fluster_renderer,
            stage_size,
            gl_context,
        })
    }

    pub fn draw_frame(
        &mut self,
        background_color: LinSrgb,
        engine: &Engine,
    ) -> Result<ImageHandle, Box<dyn Error>> {
        self.renderer.start_frame(self.stage_size.to_f32());
        self.renderer
            .set_background(lin_srgb_to_coloru(background_color));
        paint(&mut self.renderer, engine);
        self.renderer.end_frame();
        let pixels = unsafe {
            let buffer_size = self.stage_size.x() * self.stage_size.y() * 4;
            let mut target: Vec<u8> = vec![0; buffer_size as usize];
            let ptr = (&mut target).as_mut_ptr();
            // TODO: I *think* this is copying the y-axis inverted!!
            ReadPixels(
                0,
                0,
                self.stage_size.x(),
                self.stage_size.y(),
                BGRA,
                UNSIGNED_BYTE,
                ptr as *mut c_void,
            );
            target
        };
        self.gl_context.swap_buffers().unwrap();
        Ok(ImageHandle::from_pixels(
            self.stage_size.x().try_into()?,
            self.stage_size.y().try_into()?,
            pixels,
        ))
    }
}
