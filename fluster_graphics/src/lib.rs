#![deny(clippy::all)]
use fluster_core::rendering::{Coloring, Point, Renderer, Shape};
use pathfinder_canvas::{
    Canvas, CanvasFontContext, CanvasRenderingContext2D, FillStyle, LineJoin, Path2D,
};
use pathfinder_color::ColorU;
use pathfinder_content::fill::FillRule;
use pathfinder_content::pattern::Pattern;
use pathfinder_content::stroke::{LineJoin as StrokeLineJoin, StrokeStyle};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use pathfinder_gpu::Device;
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
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

fn points_to_path(points: &[Point], is_closed: bool) -> Path2D {
    let mut path = Path2D::new();
    for point in points {
        match point {
            Point::Move(to) => path.move_to(*to),
            Point::Line(to) => path.line_to(*to),
            Point::Quadratic { control, to } => path.quadratic_curve_to(*control, *to),
            Point::Bezier {
                control_1,
                control_2,
                to,
            } => path.bezier_curve_to(*control_1, *control_2, *to),
            Point::Arc {
                control,
                to,
                radius,
            } => path.arc_to(*control, *to, *radius),
        }
    }
    if is_closed {
        path.close_path();
    }
    path
}

fn stroke_path(
    canvas: &mut CanvasRenderingContext2D,
    points: &[Point],
    is_closed: bool,
    stroke_style: &StrokeStyle,
    transform: &Transform2F,
    color: ColorU,
) {
    let path = points_to_path(&points, is_closed);
    canvas.set_transform(transform);
    canvas.set_line_width(stroke_style.line_width);
    canvas.set_line_cap(stroke_style.line_cap);
    canvas.set_line_join(patch_line_join(stroke_style.line_join));
    canvas.set_stroke_style(FillStyle::Color(color));
    canvas.stroke_path(path);
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

    pub fn replace_frame_buffer(
        &mut self,
        new_dest_framebuffer: DestFramebuffer<D>,
    ) -> DestFramebuffer<D> {
        self.renderer.replace_dest_framebuffer(new_dest_framebuffer)
    }
}

impl<D> Renderer for FlusterRenderer<D>
where
    D: Device,
{
    fn start_frame(&mut self, stage_size: Vector2F) {
        self.canvas = Some(Canvas::new(stage_size).get_context_2d(self.font_context.clone()))
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
        morph_index: f32,
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
                        stroke_path(canvas, points, *is_closed, stroke_style, &transform, *color);
                    }
                }
                Shape::Fill { points, color } => {
                    if points.len() > 2 {
                        let color = if let Some(Coloring::Color(color_override)) = color_override {
                            color_override
                        } else {
                            color
                        };
                        let path = points_to_path(points, true);
                        canvas.set_transform(&transform);
                        canvas.set_fill_style(FillStyle::Color(*color));
                        canvas.fill_path(path, FillRule::Winding);
                    }
                }
                Shape::MorphPath {
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
                        let points = points
                            .iter()
                            .map(|mp| mp.to_point(morph_index))
                            .collect::<Vec<Point>>();
                        stroke_path(
                            canvas,
                            &points,
                            *is_closed,
                            stroke_style,
                            &transform,
                            *color,
                        );
                    }
                }
                Shape::MorphFill { points, color } => {
                    if points.len() > 2 {
                        let color = if let Some(Coloring::Color(color_override)) = color_override {
                            color_override
                        } else {
                            color
                        };
                        let points = points
                            .iter()
                            .map(|mp| mp.to_point(morph_index))
                            .collect::<Vec<Point>>();
                        let path = points_to_path(&points, true);
                        canvas.set_transform(&transform);
                        canvas.set_fill_style(FillStyle::Color(*color));
                        canvas.fill_path(path, FillRule::Winding);
                    }
                }
                Shape::Clip { points } => {
                    if points.len() > 2 {
                        let path = points_to_path(points, true);
                        canvas.set_transform(&transform);
                        canvas.clip_path(path, FillRule::Winding);
                    }
                }
                Shape::Group { shapes } => {
                    if let Some(Coloring::Colorings(color_override)) = color_override {
                        if color_override.len() == shapes.len() {
                            for i in 0..shapes.len() {
                                let shape = &shapes[i];
                                let color_override = &Some(color_override[i].clone());
                                self.draw_shape(
                                    &shape.shape,
                                    transform * shape.transform,
                                    color_override,
                                    morph_index,
                                )
                            }
                            return;
                        }
                    }
                    for shape in shapes {
                        self.draw_shape(
                            &shape.shape,
                            transform * shape.transform,
                            color_override,
                            morph_index,
                        )
                    }
                }
            }
        }
    }

    fn draw_raster(
        &mut self,
        pattern: &Pattern,
        view_rect: RectF,
        transform: Transform2F,
        tint: Option<ColorU>,
    ) {
        if let Some(canvas) = &mut self.canvas {
            canvas.set_transform(&transform);
            canvas.draw_subimage(
                pattern.clone(),
                view_rect,
                RectF::new(Vector2F::zero(), view_rect.size()),
            );
        }
    }
    fn end_frame(&mut self) {
        if self.canvas.is_some() {
            let canvas = mem::replace(&mut self.canvas, None).unwrap();
            let scene = SceneProxy::from_scene(canvas.into_canvas().into_scene(), RayonExecutor);
            scene.build_and_render(&mut self.renderer, BuildOptions::default());
            (self.on_frame_end)();
        }
    }
}

//TODO: how do I test this?
