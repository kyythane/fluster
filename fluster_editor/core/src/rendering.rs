#![deny(clippy::all)]
use fluster_core::rendering::{paint, Renderer as FlusterRenderer};
use fluster_graphics::FlusterRendererImpl;
use gl::{GetTextureImage, RGBA, UNSIGNED_BYTE};
use pathfinder_canvas::CanvasFontContext;
use pathfinder_color::{ColorF, ColorU};
use pathfinder_geometry::vector::Vector2I;
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_gpu::{Device, TextureFormat};
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_resources::fs::FilesystemResourceLoader;
use sdl2::video::{GLProfile, Window};
use std::ffi::c_void;

pub struct StageRenderer {
    renderer: FlusterRendererImpl<GLDevice>,
    window: Window,
    stage_size: Vector2I,
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

        let fluster_renderer = FlusterRendererImpl::new(
            font_context,
            renderer,
            Box::new(|| ()), //Box::new(move || window.gl_swap_window()),
        );
        Ok(StageRenderer {
            renderer: fluster_renderer,
            window,
            stage_size,
        })
    }

    /*TODO:
        Next steps:
        - make layer based render_data function
        - add library
        - test render to texture
        - prototype edit handles
        - ???
    */

    pub fn draw_frame(&mut self, background_color: ColorU) -> Result<Vec<u8>, String> {
        self.renderer.start_frame(self.stage_size.to_f32());
        self.renderer.set_background(background_color);
        //paint(&mut self.renderer, render_data, library);
        self.renderer.end_frame();
        self.window.gl_swap_window();
        let buffer = self.renderer.replace_frame_buffer(self.stage_size)?;
        let buffer_texture = buffer.gl_framebuffer;
        let buffer_size = {
            let buffer_size = buffer.texture.size;
            buffer_size.x() * buffer_size.y()
        };
        let texture = unsafe {
            let mut target: Vec<u8> = vec![0; buffer_size as usize];
            let ptr = (&mut target).as_mut_ptr();
            GetTextureImage(
                buffer_texture,
                0,
                RGBA,
                UNSIGNED_BYTE,
                buffer_size,
                ptr as *mut c_void,
            );
            Vec::from_raw_parts(ptr, buffer_size as usize, buffer_size as usize)
        };
        Ok(texture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let mut stage_renderer = StageRenderer::new(Vector2I::splat(100)).unwrap();
        let texture = stage_renderer
            .draw_frame(ColorU::new(254, 200, 216, 255))
            .unwrap();
        println!("{:?} {:?}", texture.len(), &texture[..16]);
        assert_eq!(2 + 2, 4);
    }
}
