#![deny(clippy::all)]
use fluster_core::rendering::{paint, RenderData, Renderer as FlusterRenderer};
use fluster_core::types::model::DisplayLibraryItem;
use fluster_graphics::FlusterRendererImpl;
use gl::{ReadPixels, BGRA, UNSIGNED_BYTE};
use pathfinder_canvas::CanvasFontContext;
use pathfinder_color::{ColorF, ColorU};
use pathfinder_geometry::vector::Vector2I;
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_resources::fs::FilesystemResourceLoader;
use sdl2::video::{GLProfile, Window};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::ffi::c_void;
use uuid::Uuid;

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

        let renderer = Renderer::new(
            GLDevice::new(GLVersion::GL3, 0),
            &FilesystemResourceLoader::locate(),
            DestFramebuffer::default(),
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
    fn compute_render_data<'a>(&self) -> RenderData<'a> {
        RenderData::new(BTreeMap::new(), HashMap::new())
    }

    pub fn draw_frame(
        &mut self,
        background_color: ColorU,
        library: &HashMap<Uuid, DisplayLibraryItem>,
    ) -> Result<Vec<u8>, String> {
        self.renderer.start_frame(self.stage_size.to_f32());
        self.renderer.set_background(background_color);
        let render_data = self.compute_render_data();
        paint(&mut self.renderer, render_data, library);
        self.renderer.end_frame();
        let texture = unsafe {
            let buffer_size = self.stage_size.x() * self.stage_size.y() * 4;
            let mut target: Vec<u8> = vec![0; buffer_size as usize];
            let ptr = (&mut target).as_mut_ptr();
            ReadPixels(
                0,
                0,
                self.stage_size.x(),
                self.stage_size.y(),
                BGRA,
                UNSIGNED_BYTE,
                ptr as *mut c_void,
            );
            Vec::from_raw_parts(ptr, buffer_size as usize, buffer_size as usize)
        };
        self.window.gl_swap_window();
        Ok(texture)
    }
}
