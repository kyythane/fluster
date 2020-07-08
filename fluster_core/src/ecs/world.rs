use super::{
    components::{Bounds, Morph, Order, RasterDisplay, Transform, Tweens, VectorDisplay},
    resources::{FrameTime, SceneGraph},
    systems::{
        ApplyMorphTweens, ApplyTransformTweens, UpdateBounds, UpdateTweens, UpdateWorldTransform,
    },
};
use specs::{Builder, Dispatcher, DispatcherBuilder, World, WorldExt};

pub struct ECS<'a, 'b> {
    world: World,
    dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> ECS<'a, 'b> {
    pub fn step(&mut self, frame_time: FrameTime) {
        self.world.insert(frame_time);
        self.dispatcher.dispatch(&mut self.world);
        self.world.maintain();
    }
}

pub fn build_world<'a, 'b>() -> ECS<'a, 'b> {
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Bounds>();
    world.register::<Morph>();
    world.register::<Order>();
    world.register::<RasterDisplay>();
    world.register::<VectorDisplay>();
    world.register::<Tweens>();
    let root = world.create_entity().with(Transform::default()).build();
    world.insert(SceneGraph::new(root));

    let dispatcher = DispatcherBuilder::new()
        .with(ApplyTransformTweens, "apply_transform_tweens", &[])
        .with(ApplyMorphTweens, "apply_morph_tweens", &[])
        .with(
            UpdateWorldTransform,
            "update_world_transform",
            &["apply_transform_tweens"],
        )
        .with(UpdateBounds, "update_bounds", &["update_world_transform"])
        .with(
            UpdateTweens,
            "update_tweens",
            &["apply_transform_tweens", "apply_morph_tweens"],
        )
        .build();
    ECS { world, dispatcher }
}
