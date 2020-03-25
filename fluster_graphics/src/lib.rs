use fluster_core::rendering::{Bitmap, Coloring, Renderer, Shape};
use pathfinder_canvas::{CanvasFontContext, CanvasRenderingContext2D, FillStyle, LineJoin, Path2D};
use pathfinder_color::ColorU;
use pathfinder_content::fill::FillRule;
use pathfinder_content::stroke::LineJoin as StrokeLineJoin;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use pathfinder_gpu::Device;
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::RendererOptions;
use pathfinder_renderer::gpu::renderer::Renderer as PathfinderRenderer;
use pathfinder_renderer::options::BuildOptions;
use std::mem;

fn patch_line_join(j: StrokeLineJoin) -> LineJoin {
    match j {
        StrokeLineJoin::Miter(_) => LineJoin::Miter,
        StrokeLineJoin::Bevel => LineJoin::Bevel,
        StrokeLineJoin::Round => LineJoin::Round,
    }
}

fn points_to_path(points: &Vec<Vector2F>, close_path: bool) -> Path2D {
    let mut path = Path2D::new();
    let mut points = points.iter();
    path.move_to(*points.next().unwrap());
    for point in points {
        path.line_to(*point);
    }
    if close_path {
        path.close_path();
    }
    path
}

pub struct FlusterRenderer<D>
where
    D: Device,
{
    font_context: CanvasFontContext,
    renderer: PathfinderRenderer<D>,
    canvas: Option<CanvasRenderingContext2D>,
    on_frame_end: Box<dyn Fn() -> ()>,
}

impl<D> FlusterRenderer<D>
where
    D: Device,
{
    pub fn new(
        font_context: CanvasFontContext,
        renderer: PathfinderRenderer<D>,
        on_frame_end: Box<dyn Fn() -> ()>,
    ) -> FlusterRenderer<D> {
        FlusterRenderer {
            font_context,
            canvas: None,
            renderer,
            on_frame_end,
        }
    }
}

impl<D> Renderer for FlusterRenderer<D>
where
    D: Device,
{
    fn start_frame(&mut self, stage_size: Vector2F) {
        self.canvas = Some(CanvasRenderingContext2D::new(
            self.font_context.clone(),
            stage_size,
        ))
    }
    fn set_background(&mut self, color: ColorU) {
        self.renderer.set_options(RendererOptions {
            background_color: Some(color.to_f32()),
        });
    }
    fn draw_shape(
        &mut self,
        shape: &Shape,
        transform: Transform2F,
        color_override: &Option<Coloring>,
    ) {
        if let Some(canvas) = &mut self.canvas {
            match shape {
                Shape::Path {
                    points,
                    color,
                    stroke_style,
                    is_closed,
                } => {
                    if points.len() > 1 {
                        let color = if let Some(Coloring::Color(color_override)) = color_override {
                            color_override
                        } else {
                            color
                        };
                        let path = points_to_path(points, *is_closed);
                        canvas.set_current_transform(&transform);
                        canvas.set_line_width(stroke_style.line_width);
                        canvas.set_line_cap(stroke_style.line_cap);
                        canvas.set_line_join(patch_line_join(stroke_style.line_join));
                        canvas.set_stroke_style(FillStyle::Color(*color));
                        canvas.stroke_path(path);
                    }
                }
                Shape::FillPath { points, color } => {
                    if points.len() > 2 {
                        let color = if let Some(Coloring::Color(color_override)) = color_override {
                            color_override
                        } else {
                            color
                        };
                        let path = points_to_path(points, true);
                        canvas.set_current_transform(&transform);
                        canvas.set_fill_style(FillStyle::Color(*color));
                        canvas.fill_path(path, FillRule::Winding);
                    }
                }
                Shape::Clip { points } => {
                    if points.len() > 2 {
                        let path = points_to_path(points, true);
                        canvas.set_current_transform(&transform);
                        canvas.clip_path(path, FillRule::Winding);
                    }
                }
                Shape::Group { shapes } => {
                    for shape in shapes {
                        self.draw_shape(shape, transform, color_override)
                    }
                }
            }
        }
    }

    fn draw_bitmap(
        &mut self,
        _bitmap: &Bitmap,
        _view_rect: RectF,
        _transform: Transform2F,
        _tint: Option<ColorU>,
    ) {
        /*TODO:
            This will require with dealing with GL directly
            A batching scheme for both shapes and textures will also need to be set up.
        */
    }
    fn end_frame(&mut self) {
        if self.canvas.is_some() {
            let canvas = mem::replace(&mut self.canvas, None).unwrap();
            let scene = SceneProxy::from_scene(canvas.into_scene(), RayonExecutor);
            scene.build_and_render(&mut self.renderer, BuildOptions::default());
            (self.on_frame_end)();
        }
    }
}

//TODO: how do I test this?
