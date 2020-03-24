use fluster_core::rendering::{Bitmap, Renderer, Shape};
use pathfinder_color::ColorU;
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;

pub struct FlusterRenderer {}

impl Renderer for FlusterRenderer {
    fn set_background(&self, color: ColorU) {}
    fn draw_shape(&self, shape: &Shape, transform: Transform2F, color_override: Option<ColorU>) {}
    fn draw_bitmap(
        &self,
        _bitmap: &Bitmap,
        _view_rect: RectF,
        _transform: Transform2F,
        _tint: Option<ColorU>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
