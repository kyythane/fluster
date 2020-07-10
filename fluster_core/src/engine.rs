use crate::ecs::{
    components::{Bounds, Coloring, Display, Layer, Morph, Order, Transform, Tweens, ViewRect},
    resources::{ContainerMapping, FrameTime, Library, QuadTrees, SceneGraph},
    systems::{
        ApplyMorphTweens, ApplyTransformTweens, ApplyViewRectTweens, UpdateBounds, UpdateQuadTree,
        UpdateTweens, UpdateWorldTransform,
    },
};
use specs::{shred::Fetch, Builder, Dispatcher, DispatcherBuilder, World, WorldExt};
use uuid::Uuid;

pub struct Engine<'a, 'b> {
    world: World,
    dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> Engine<'a, 'b> {
    pub fn new(root_container_id: Uuid, library: Library) -> Self {
        let mut world = World::new();
        //Register components
        world.register::<Transform>();
        world.register::<Bounds>();
        world.register::<Morph>();
        world.register::<Order>();
        world.register::<Display>();
        world.register::<Tweens>();
        world.register::<Layer>();
        world.register::<Coloring>();
        world.register::<ViewRect>();

        // Setup resources
        let root = world.create_entity().with(Transform::default()).build();
        world.insert(SceneGraph::new(root));
        world.write_resource::<QuadTrees>();
        let mut container_mapping = ContainerMapping::default();
        container_mapping.add_container(root_container_id, root);
        world.insert(container_mapping);
        world.insert(library);

        // Setup systems
        let dispatcher = DispatcherBuilder::new()
            .with(ApplyTransformTweens, "apply_transform_tweens", &[])
            .with(ApplyMorphTweens, "apply_morph_tweens", &[])
            .with(ApplyViewRectTweens, "apply_view_rect_tweens", &[])
            .with(
                UpdateWorldTransform,
                "update_world_transform",
                &["apply_transform_tweens"],
            )
            .with(UpdateBounds, "update_bounds", &["update_world_transform"])
            .with(UpdateQuadTree, "update_quad_tree", &["update_bounds"])
            .with(
                UpdateTweens,
                "update_tweens",
                &["apply_transform_tweens", "apply_morph_tweens"],
            )
            .build();
        Engine { world, dispatcher }
    }

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

    pub fn add_container(&mut self) {
        let mut scene_graph = self.world.write_resource::<SceneGraph>();
        let mut container_mapping = self.world.write_resource::<ContainerMapping>();
    }

    pub fn remove_container(&mut self, container_id: &Uuid) {
        let mut scene_graph = self.world.write_resource::<SceneGraph>();
        let mut container_mapping = self.world.write_resource::<ContainerMapping>();
        if let Some(entity) = container_mapping.get_entity(container_id) {
            container_mapping.remove_container(container_id);
            scene_graph.remove_entity(entity);
            self.world.delete_entity(*entity);
        }
    }

    pub fn remove_container_and_children(&mut self, container_id: &Uuid) {
        let mut scene_graph = self.world.write_resource::<SceneGraph>();
        let mut container_mapping = self.world.write_resource::<ContainerMapping>();
        if let Some(entity) = container_mapping.get_entity(container_id) {
            scene_graph
                .remove_entity_and_children(entity)
                .into_iter()
                .for_each(|entity| {
                    container_mapping.remove_entity(&entity);
                    self.world.delete_entity(entity);
                });
        }
    }
}
