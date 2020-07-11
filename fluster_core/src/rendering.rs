#![deny(clippy::all)]
use super::types::{coloring::Coloring, shapes::Shape};
use crate::engine::{Engine, LibraryItem};
use palette::{IntoComponent, LinSrgb, LinSrgba};
use pathfinder_color::ColorU;
use pathfinder_content::pattern::Pattern;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use std::sync::Arc;

pub fn lin_srgb_to_coloru(rgb: LinSrgb) -> ColorU {
    let components = rgb.into_components();
    ColorU::new(
        components.0.into_component(),
        components.1.into_component(),
        components.2.into_component(),
        255,
    )
}

pub fn lin_srgba_to_coloru(rgba: LinSrgba) -> ColorU {
    let components = rgba.into_components();
    ColorU::new(
        components.0.into_component(),
        components.1.into_component(),
        components.2.into_component(),
        components.3.into_component(),
    )
}
pub trait Renderer {
    fn start_frame(&mut self, stage_size: Vector2F);
    fn set_background(&mut self, color: ColorU);
    fn draw_shape(
        &mut self,
        shape: Arc<Shape>,
        transform: Transform2F,
        color_override: Option<Coloring>,
        morph_index: f32,
    );
    fn draw_raster(
        &mut self,
        pattern: Arc<Pattern>,
        view_rect: Option<RectF>,
        transform: Transform2F,
        tint: Option<Coloring>,
    );
    fn end_frame(&mut self);
}

pub fn paint(renderer: &mut impl Renderer, engine: &Engine) {
    for drawable_item in engine.get_drawable_items() {
        match drawable_item.library_item {
            LibraryItem::Vector(shape) => {
                renderer.draw_shape(
                    shape,
                    drawable_item.transform,
                    drawable_item.coloring,
                    drawable_item.morph,
                );
            }
            LibraryItem::Raster(pattern) => {
                renderer.draw_raster(
                    pattern,
                    drawable_item.view_rect,
                    drawable_item.transform,
                    drawable_item.coloring,
                );
            }
        }
    }
}
