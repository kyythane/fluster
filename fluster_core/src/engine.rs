use crate::ecs::{
    components::{Bounds, Layer, Morph, Order, RasterDisplay, Transform, Tweens, VectorDisplay},
    resources::{ContainerMapping, FrameTime, Library, QuadTrees, SceneGraph},
    systems::{
        ApplyMorphTweens, ApplyTransformTweens, UpdateBounds, UpdateTweens, UpdateWorldTransform,
    },
};
use specs::{shred::Fetch, Builder, Dispatcher, DispatcherBuilder, World, WorldExt};
use uuid::Uuid;

pub struct Engine<'a, 'b> {
    world: World,
    dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> Engine<'a, 'b> {
    pub fn update(&mut self, frame_time: FrameTime) {
        self.world.insert(frame_time);
        self.dispatcher.dispatch(&mut self.world);
        self.world.maintain();
    }

    pub fn get_scene_graph(&self) -> Fetch<SceneGraph> {
        self.world.read_resource::<SceneGraph>()
    }

    pub fn get_container_mapping(&self) -> Fetch<ContainerMapping> {
        self.world.read_resource::<ContainerMapping>()
    }

    pub fn get_root_container_id(&self) -> Uuid {
        let scene_graph = self.get_scene_graph();
        let container_mapping = self.get_container_mapping();
        *container_mapping.get_container(scene_graph.root()).unwrap()
    }
}

pub fn build_ecs<'a, 'b>(root_container_id: Uuid) -> Engine<'a, 'b> {
    let mut world = World::new();
    //Register components
    world.register::<Transform>();
    world.register::<Bounds>();
    world.register::<Morph>();
    world.register::<Order>();
    world.register::<RasterDisplay>();
    world.register::<VectorDisplay>();
    world.register::<Tweens>();
    world.register::<Layer>();

    // Setup resources
    let root = world.create_entity().with(Transform::default()).build();
    world.insert(SceneGraph::new(root));
    world.write_resource::<QuadTrees>();
    let mut container_mapping = ContainerMapping::default();
    container_mapping.add_container(root_container_id, root);
    world.insert(container_mapping);
    world.write_resource::<Library>();

    // Setup systems
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
    Engine { world, dispatcher }
}
