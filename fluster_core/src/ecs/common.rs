use super::{
    components::{BoundsSource, Display, DisplayKind, Morph, ViewRect},
    resources::Library,
};
use pathfinder_geometry::{rect::RectF, transform2d::Transform2F, vector::Vector2F};

pub fn recompute_bounds(
    source: &BoundsSource,
    transform: Transform2F,
    display: Option<&Display>,
    view_rect: Option<&ViewRect>,
    morph: Option<&Morph>,
    library: &Library,
) -> RectF {
    match source {
        BoundsSource::Display => match display {
            Some(Display(id, DisplayKind::Vector)) => {
                let shape = library.get_shape(id).unwrap();
                shape.compute_bounding(&transform, morph.unwrap_or(&Morph(0.0)).0)
            }
            Some(Display(id, DisplayKind::Raster)) => {
                let pattern = library.get_texture(id).unwrap();
                let (o, lr) = view_rect
                    .and_then(|ViewRect(rect)| Some((rect.origin(), rect.lower_right())))
                    .unwrap_or_else(|| (Vector2F::zero(), pattern.size().to_f32()));
                let o = transform * o;
                let lr = transform * lr;
                RectF::from_points(o.min(lr), o.max(lr))
            }
            None => {
                panic!("Attmpting to compute the bounds of an entity without an attached display")
            }
        },
        BoundsSource::Defined(rect) => {
            let o = transform * rect.origin();
            let lr = transform * rect.lower_right();
            RectF::from_points(o.min(lr), o.max(lr))
        }
    }
}
