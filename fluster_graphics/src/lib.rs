#![deny(clippy::all)]
use fluster_core::rendering::{lin_srgba_to_coloru, Renderer};
use fluster_core::types::{
    coloring::Coloring,
    shapes::{Edge, Shape},
};
use pathfinder_canvas::{Canvas, CanvasFontContext, CanvasRenderingContext2D, FillStyle, LineJoin};
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
use pathfinder_renderer::gpu::options::RendererLevel;
use pathfinder_renderer::gpu::renderer::Renderer as PathfinderRenderer;
use pathfinder_renderer::options::BuildOptions;
use std::{mem, sync::Arc};

fn patch_line_join(j: StrokeLineJoin) -> LineJoin {
    match j {
        StrokeLineJoin::Miter(_) => LineJoin::Miter,
        StrokeLineJoin::Bevel => LineJoin::Bevel,
        StrokeLineJoin::Round => LineJoin::Round,
    }
}

fn stroke_path(
    canvas: &mut CanvasRenderingContext2D,
    edges: impl Iterator<Item = Edge>,
    stroke_style: &StrokeStyle,
    transform: &Transform2F,
    color: ColorU,
) {
    let path = Edge::edges_to_path(edges);
    canvas.set_transform(transform);
    canvas.set_line_width(stroke_style.line_width);
    canvas.set_line_cap(stroke_style.line_cap);
    canvas.set_line_join(patch_line_join(stroke_style.line_join));
    canvas.set_stroke_style(FillStyle::Color(color));
    canvas.stroke_path(path);
}

pub struct FlusterRendererImpl<D>
where
    D: Device,
{
    font_context: CanvasFontContext,
    renderer: PathfinderRenderer<D>,
    canvas: Option<CanvasRenderingContext2D>,
    on_frame_end: Box<dyn Fn() -> ()>,
}

impl<D> FlusterRendererImpl<D>
where
    D: Device,
{
    pub fn new(
        font_context: CanvasFontContext,
        renderer: PathfinderRenderer<D>,
        on_frame_end: Box<dyn Fn() -> ()>,
    ) -> FlusterRendererImpl<D> {
        FlusterRendererImpl {
            font_context,
            canvas: None,
            renderer,
            on_frame_end,
        }
    }
}

impl<D> FlusterRendererImpl<D>
where
    D: Device,
{
    fn handle_draw_shape(
        &mut self,
        shape: &Shape,
        transform: Transform2F,
        color_override: Option<Coloring>,
        morph_index: f32,
    ) {
        if let Some(canvas) = &mut self.canvas {
            match shape {
                Shape::Path {
                    edges,
                    color,
                    stroke_style,
                } => {
                    if edges.len() > 1 {
                        let color = if let Some(Coloring::Color(color_override)) = color_override {
                            color_override
                        } else {
                            *color
                        };
                        stroke_path(
                            canvas,
                            edges.iter().map(|e| *e),
                            stroke_style,
                            &transform,
                            lin_srgba_to_coloru(color),
                        );
                    }
                }
                Shape::Fill { edges, color } => {
                    if edges.len() > 2 {
                        let color = if let Some(Coloring::Color(color_override)) = color_override {
                            color_override
                        } else {
                            *color
                        };
                        let path = Edge::edges_to_path(edges.iter().map(|e| *e));
                        canvas.set_transform(&transform);
                        canvas.set_fill_style(FillStyle::Color(lin_srgba_to_coloru(color)));
                        canvas.fill_path(path, FillRule::Winding);
                    }
                }
                Shape::MorphPath {
                    edges,
                    color,
                    stroke_style,
                } => {
                    if edges.len() > 1 {
                        let color = if let Some(Coloring::Color(color_override)) = color_override {
                            color_override
                        } else {
                            *color
                        };
                        let edges = edges.iter().map(|mp| mp.to_edge(morph_index));
                        stroke_path(
                            canvas,
                            edges,
                            stroke_style,
                            &transform,
                            lin_srgba_to_coloru(color),
                        );
                    }
                }
                Shape::MorphFill { edges, color } => {
                    if edges.len() > 2 {
                        let color = if let Some(Coloring::Color(color_override)) = color_override {
                            color_override
                        } else {
                            *color
                        };
                        let edges = edges.iter().map(|mp| mp.to_edge(morph_index));
                        let path = Edge::edges_to_path(edges);
                        canvas.set_transform(&transform);
                        canvas.set_fill_style(FillStyle::Color(lin_srgba_to_coloru(color)));
                        canvas.fill_path(path, FillRule::Winding);
                    }
                }
                Shape::Clip { edges } => {
                    if edges.len() > 2 {
                        let path = Edge::edges_to_path(edges.iter().map(|e| *e));
                        canvas.set_transform(&transform);
                        canvas.clip_path(path, FillRule::Winding);
                    }
                }
                Shape::Group { shapes } => {
                    if let Some(Coloring::Colorings(mut color_overrides)) = color_override {
                        for (color, shape) in color_overrides.drain(..).zip(shapes) {
                            self.handle_draw_shape(
                                &shape.shape,
                                transform * shape.transform,
                                Some(color),
                                morph_index,
                            )
                        }
                    } else {
                        for shape in shapes {
                            self.handle_draw_shape(
                                &shape.shape,
                                transform * shape.transform,
                                None,
                                morph_index,
                            )
                        }
                    }
                }
            }
        }
    }
}

impl<D> Renderer for FlusterRendererImpl<D>
where
    D: Device,
{
    //TODO: handle stage_size changing
    fn start_frame(&mut self, stage_size: Vector2F) {
        self.canvas = Some(Canvas::new(stage_size).get_context_2d(self.font_context.clone()))
    }
    fn set_background(&mut self, color: ColorU) {
        self.renderer.options_mut().background_color = Some(color.to_f32());
    }

    fn draw_shape(
        &mut self,
        shape: Arc<Shape>,
        transform: Transform2F,
        color_override: Option<Coloring>,
        morph_index: f32,
    ) {
        self.handle_draw_shape(&*shape, transform, color_override, morph_index);
    }

    fn draw_raster(
        &mut self,
        pattern: Arc<Pattern>,
        view_rect: Option<RectF>,
        transform: Transform2F,
        _tint: Option<Coloring>, //TODO: tinting
    ) {
        let view_rect = view_rect
            .and_then(|rect| Some(rect))
            .unwrap_or(RectF::from_points(
                Vector2F::zero(),
                pattern.size().to_f32(),
            ));
        if let Some(canvas) = &mut self.canvas {
            canvas.set_transform(&transform);
            canvas.draw_subimage(
                (*pattern).clone(),
                view_rect,
                RectF::new(Vector2F::zero(), view_rect.size()),
            );
        }
    }
    fn end_frame(&mut self) {
        if self.canvas.is_some() {
            let canvas = mem::replace(&mut self.canvas, None).unwrap();
            let mut scene = SceneProxy::from_scene(
                canvas.into_canvas().into_scene(),
                RendererLevel::D3D9,
                RayonExecutor,
            );
            scene.build_and_render(&mut self.renderer, BuildOptions::default());
            (self.on_frame_end)();
        }
    }
}

//TODO: how do I test this?
