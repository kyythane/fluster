use super::resources::QuadTreeLayer;
use crate::tween::PropertyTween;
use crate::types::{basic::LibraryId, coloring::Coloring};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use specs::{
    storage::{BTreeStorage, DenseVecStorage, VecStorage},
    Component, FlaggedStorage,
};
use std::collections::HashSet;
#[derive(Debug, Copy, Clone, Default)]
pub struct LocalTransform(pub Transform2F);

impl Component for LocalTransform {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

#[derive(Debug, Copy, Clone, Default)]
pub struct WorldTransform(pub Transform2F);

impl Component for WorldTransform {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

#[derive(Debug)]
pub struct Bounds {
    pub bounds: RectF,
    pub source: BoundsSource,
}

impl Component for Bounds {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

#[derive(Debug)]
pub enum BoundsSource {
    Display,
    Defined(RectF),
}

#[derive(Debug, Default)]
pub struct Layer {
    pub quad_trees: HashSet<QuadTreeLayer>,
}

impl Component for Layer {
    type Storage = FlaggedStorage<Self, BTreeStorage<Self>>;
}

#[derive(Component, Debug)]
#[storage(DenseVecStorage)]
pub struct Display(pub LibraryId, pub DisplayKind);

#[derive(Clone, Copy, Debug)]
pub enum DisplayKind {
    Raster,
    Vector,
}

#[derive(Component, Debug)]
#[storage(BTreeStorage)]
pub struct ViewRect(pub RectF);

impl Component for Coloring {
    type Storage = BTreeStorage<Self>;
}

#[derive(Component, Clone, Copy, Default, Debug)]
#[storage(VecStorage)]
pub struct Order(pub i8);

#[derive(Debug, Default, Copy, Clone)]
pub struct Morph(pub f32);

impl Component for Morph {
    type Storage = FlaggedStorage<Self, BTreeStorage<Self>>;
}

#[derive(Component, Debug)]
#[storage(BTreeStorage)]
pub struct Tweens(pub Vec<PropertyTween>);
