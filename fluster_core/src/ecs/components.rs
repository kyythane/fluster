use super::resources::QuadTreeLayer;
use crate::tween::PropertyTween;
use crate::types::coloring::Coloring;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use specs::{
    storage::{BTreeStorage, DenseVecStorage, VecStorage},
    Component,
};
use std::collections::HashSet;
use uuid::Uuid;
#[derive(Component, Debug, Copy, Clone)]
#[storage(VecStorage)]
pub struct Transform {
    pub local: Transform2F,
    pub world: Transform2F,
    pub dirty: bool,
}

// Default dirty true
impl Default for Transform {
    fn default() -> Self {
        Self {
            local: Transform2F::default(),
            world: Transform2F::default(),
            dirty: true,
        }
    }
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Bounds {
    pub bounds: RectF,
    pub source: BoundsSource,
    pub dirty: bool,
}

#[derive(Debug)]
pub enum BoundsSource {
    Display,
    Defined(RectF),
}

#[derive(Component, Debug, Default)]
#[storage(BTreeStorage)]
pub struct Layer {
    pub quad_trees: HashSet<QuadTreeLayer>,
}

#[derive(Component, Debug)]
#[storage(DenseVecStorage)]
pub struct Display(pub Uuid, pub DisplayKind);

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

#[derive(Component, Debug, Default, Copy, Clone)]
#[storage(VecStorage)]
pub struct Morph(pub f32);

#[derive(Component, Debug)]
#[storage(BTreeStorage)]
pub struct Tweens(pub Vec<PropertyTween>);
