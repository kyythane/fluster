use crate::types::shapes::Coloring;
use crate::{runner::QuadTreeLayer, tween::PropertyTween};
use pathfinder_color::ColorU;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use specs::{
    storage::{BTreeStorage, DenseVecStorage, VecStorage},
    Component,
};
use std::collections::HashSet;
use uuid::Uuid;
#[derive(Component, Debug, Default)]
#[storage(VecStorage)]
pub struct Transform {
    pub local: Transform2F,
    pub world: Transform2F,
    pub touched: bool,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Bounds {
    pub bounds: RectF,
    pub source: BoundsSource,
}

#[derive(Debug)]
pub enum BoundsSource {
    Display,
    Defined(RectF),
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Layer {
    pub quad_trees: HashSet<QuadTreeLayer>,
}

#[derive(Component, Debug)]
#[storage(DenseVecStorage)]
pub struct VectorDisplay {
    pub target: Uuid,
    pub coloring: Option<Coloring>,
}

#[derive(Component, Debug)]
#[storage(DenseVecStorage)]
pub struct RasterDisplay {
    pub target: Uuid,
    pub view_rect: RectF,
    pub tint: Option<ColorU>,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Order(pub i8);

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Morph(pub f32);

#[derive(Component, Debug)]
#[storage(BTreeStorage)]
pub struct Tweens(pub Vec<PropertyTween>);
