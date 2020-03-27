/*use fluster_core::actions::{
    Action, ActionList, EntityDefinition, EntityUpdateDefinition, PartDefinition,
    PartUpdateDefinition,
};
use fluster_core::rendering::{AugmentedShape, Coloring, Point, Shape};
use fluster_core::runner;
use fluster_core::tween::Easing;
use fluster_core::types::ScaleRotationTranslation;
use fluster_graphics::FlusterRenderer;
use pathfinder_canvas::CanvasFontContext;
use pathfinder_color::{ColorF, ColorU};
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_resources::fs::FilesystemResourceLoader;
use std::f32::consts::PI;
use uuid::Uuid;*/

use piston_window::{PistonWindow, WindowSettings};

const WINDOW_WIDTH: f64 = 1280.0;
const WINDOW_HEIGHT: f64 = 760.0;
const STAGE_WIDTH: f32 = 800.0;
const STAGE_HEIGHT: f32 = 600.0;

fn main() {
    let mut window: PistonWindow =
        WindowSettings::new("Fluster Editor v0.1.0", [WINDOW_WIDTH, WINDOW_HEIGHT])
            .build()
            .unwrap();
    while let Some(event) = window.next() {
        window.draw_2d(&event, |c, g, _| {});
    }
}
