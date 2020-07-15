use fluster_core::{
    engine::Engine,
    rendering::{lin_srgb_to_coloru, paint, Renderer as FlusterRenderer},
};
use fluster_graphics::FlusterRendererImpl;
use palette::LinSrgb;
use pathfinder_canvas::CanvasFontContext;
use pathfinder_color::ColorF;
use pathfinder_geometry::vector::Vector2I;
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_gpu::{Device, TextureFormat};
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererMode, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_resources::embedded::EmbeddedResourceLoader;

pub struct StageRenderer {
    renderer: FlusterRendererImpl<GLDevice>,
    stage_size: Vector2I,
}

impl StageRenderer {
    pub fn new(stage_size: Vector2I) -> Result<StageRenderer, String> {
        let device = GLDevice::new(GLVersion::GL3, 0);
        let renderer_mode = RendererMode::default_for_device(&device);
        let renderer =
            Renderer::new(
                device,
                &EmbeddedResourceLoader,
                renderer_mode,
                RendererOptions {
                    dest: DestFramebuffer::Other(device.create_framebuffer(
                        device.create_texture(TextureFormat::RGBA8, stage_size),
                    )),
                    background_color: Some(ColorF::white()),
                    show_debug_ui: false,
                },
            );

        let font_context = CanvasFontContext::from_system_source();

        let fluster_renderer = FlusterRendererImpl::new(font_context, renderer, Box::new(|| ()));
        Ok(StageRenderer {
            renderer: fluster_renderer,
            stage_size,
        })
    }

    pub fn draw_frame(&mut self, background_color: LinSrgb, engine: &Engine) {
        self.renderer.start_frame(self.stage_size.to_f32());
        self.renderer
            .set_background(lin_srgb_to_coloru(background_color));
        paint(&mut self.renderer, engine);
        self.renderer.end_frame();
    }
}
