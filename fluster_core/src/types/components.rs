use super::shapes::Coloring;
use crate::{runner::QuadTreeLayer, tween::PropertyTween};
use pathfinder_color::ColorU;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use reduce::Reduce;
use specs::{
    prelude::*,
    storage::{BTreeStorage, DenseVecStorage, VecStorage},
    Component,
};
use std::collections::HashSet;
use uuid::Uuid;
#[derive(Component, Debug)]
#[storage(VecStorage)]
struct Transform {
    local: Transform2F,
    world: Transform2F,
    touched: bool,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
struct Bounds {
    source: BoundsSource,
}

#[derive(Debug)]
enum BoundsSource {
    Display,
    Defined(RectF),
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
struct Layer {
    quad_trees: HashSet<QuadTreeLayer>,
}

#[derive(Component, Debug)]
#[storage(DenseVecStorage)]
struct VectorDisplay {
    target: Uuid,
    coloring: Option<Coloring>,
}

#[derive(Component, Debug)]
#[storage(DenseVecStorage)]
struct RasterDisplay {
    target: Uuid,
    view_rect: RectF,
    tint: Option<ColorU>,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
struct Order(i8);

#[derive(Component, Debug)]
#[storage(VecStorage)]
struct Morph(f32);

#[derive(Component, Debug)]
#[storage(BTreeStorage)]
struct Tween(Vec<PropertyTween>);

struct ApplyTransformTweens;

impl<'a> System<'a> for ApplyTransformTweens {
    type SystemData = (WriteStorage<'a, Transform>, ReadStorage<'a, Tween>);

    fn run(&mut self, (mut transform_storage, tween_storage): Self::SystemData) {
        for (transform, tweens) in (&mut transform_storage, &tween_storage).join() {
            //transform.local = tweens.iter().filter(|tween| if let (PropertyTween::))
        }
    }
}
