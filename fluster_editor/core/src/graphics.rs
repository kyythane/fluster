#![deny(clippy::all)]
use fluster_graphics::FlusterRenderer;
use pathfinder_canvas::CanvasFontContext;
use pathfinder_color::ColorF;
use pathfinder_geometry::vector::Vector2I;
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_gpu::{Device, TextureFormat};
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_resources::fs::FilesystemResourceLoader;
use sdl2::video::{GLProfile, Window};

pub struct StageRenderer {
    renderer: FlusterRenderer<GLDevice>,
    window: Window,
}

impl StageRenderer {
    //TODO: maybe make a proper type for error here?
    pub fn new(stage_size: Vector2I) -> Result<StageRenderer, String> {
        let sdl_context = sdl2::init()?;
        let video = sdl_context.video()?;
        let gl_attributes = video.gl_attr();
        gl_attributes.set_context_profile(GLProfile::Core);
        gl_attributes.set_context_version(3, 3);
        let window = video
            .window(
                "stage render target",
                stage_size.x() as u32,
                stage_size.y() as u32,
            )
            .hidden()
            .opengl()
            .build();
        let window = match window {
            Ok(win) => win,
            Err(window_error) => return Err(window_error.to_string()),
        };
        let gl_context = window.gl_create_context()?;
        gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
        window.gl_make_current(&gl_context)?;

        let device = GLDevice::new(GLVersion::GL3, 0);
        let pathfinder_framebuffer =
            device.create_framebuffer(device.create_texture(TextureFormat::RGBA8, stage_size));
        let renderer = Renderer::new(
            device,
            &FilesystemResourceLoader::locate(),
            DestFramebuffer::Other(pathfinder_framebuffer),
            RendererOptions {
                background_color: Some(ColorF::white()),
            },
        );

        let font_context = CanvasFontContext::from_system_source();

        let fluster_renderer = FlusterRenderer::new(
            font_context,
            renderer,
            Box::new(|| ()), //Box::new(move || window.gl_swap_window()),
        );
        Ok(StageRenderer {
            renderer: fluster_renderer,
            window,
        })
    }
}
