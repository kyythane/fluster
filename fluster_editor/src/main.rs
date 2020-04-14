mod rendering;
use pathfinder_color::ColorU;
use pathfinder_geometry::vector::Vector2I;
use std::collections::HashMap;

use rendering::StageRenderer;

fn main() {
    let mut stage_renderer = StageRenderer::new(Vector2I::splat(100)).unwrap();
    let library = HashMap::new();
    let texture = stage_renderer
        .draw_frame(ColorU::new(254, 200, 216, 255), &library)
        .unwrap();
    println!("{:?} {:?}", texture.len(), &texture[..16]);
}
