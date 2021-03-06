use fluster_core::{
    engine::Engine,
    rendering::{lin_srgb_to_coloru, paint, Renderer as FlusterRenderer},
};
use fluster_graphics::FlusterRendererImpl;
use gl::{ReadPixels, BGRA, UNSIGNED_BYTE};
use iced::image::Handle as ImageHandle;
use palette::LinSrgb;
use pathfinder_canvas::CanvasFontContext;
use pathfinder_color::ColorF;
use pathfinder_geometry::vector::Vector2I;
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererMode, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_resources::embedded::EmbeddedResourceLoader;
use sdl2::video::{GLContext, GLProfile, Window};
use std::convert::TryInto;
use std::error::Error;
use std::ffi::c_void;

/*
 *   Note: This is kinda a hack until there is a cleaner way to use pathfinder and iced together.
 */

pub struct StageRenderer {
    renderer: FlusterRendererImpl<GLDevice>,
    window: Window,
    stage_size: Vector2I,
    //Need to keep gl_context around so it doesn't get freed, but we don't *actually* need it for anything
    #[allow(unused_variables, dead_code)]
    gl_context: GLContext,
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
            .window("Stage", stage_size.x() as u32, stage_size.y() as u32)
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
            window,
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
        self.window.gl_swap_window();
        Ok(ImageHandle::from_pixels(
            self.stage_size.x().try_into()?,
            self.stage_size.y().try_into()?,
            pixels,
        ))
    }
}
