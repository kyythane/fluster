#![deny(clippy::all)]
use fluster_graphics::FlusterRenderer;
use pathfinder_geometry::vector::Vector2I;
use sdl2::video::GLProfile;

pub struct StageRenderer<D>
where
    D: Device,
{
    renderer: FlusterRenderer<D>,
}

impl<D> StageRenderer<D>
where
    D: Device,
{
    pub fn new(stage_size: Vector2I) -> StageRenderer<D> {
        let sdl_context = sdl2::init().unwrap();
        let video = sdl_context.video().unwrap();
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
            .build()
            .unwrap();
        let gl_context = window.gl_create_context().unwrap();
        gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
        window.gl_make_current(&gl_context).unwrap();
    }
}
