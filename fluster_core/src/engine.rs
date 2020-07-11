use crate::{
    actions::{ContainerCreationDefintition, ContainerUpdateDefintition},
    ecs::{
        components::{
            Bounds, Display, DisplayKind, Layer, Morph, Order, Transform, Tweens, ViewRect,
        },
        resources::{ContainerMapping, FrameTime, Library, QuadTrees, SceneGraph},
        systems::{
            ApplyColoringTweens, ApplyMorphTweens, ApplyTransformTweens, ApplyViewRectTweens,
            UpdateBounds, UpdateQuadTree, UpdateTweens, UpdateWorldTransform,
        },
    },
    types::{coloring::Coloring, shapes::Shape},
};
use pathfinder_content::pattern::Pattern;
use pathfinder_geometry::{rect::RectF, transform2d::Transform2F};
use specs::{
    shred::{Fetch, FetchMut},
    Builder, Dispatcher, DispatcherBuilder, Entity, Join, World, WorldExt,
};
use std::collections::{HashMap, VecDeque};
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
            .with(ApplyColoringTweens, "apply_coloring_tweens", &[])
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

    pub fn get_library(&self) -> Fetch<Library> {
        self.world.read_resource::<Library>()
    }

    pub fn get_library_mut(&mut self) -> FetchMut<Library> {
        self.world.write_resource::<Library>()
    }

    pub fn get_root_container_id(&self) -> Uuid {
        let scene_graph = self.get_scene_graph();
        let container_mapping = self.get_container_mapping();
        *container_mapping.get_container(scene_graph.root()).unwrap()
    }

    pub fn create_container(
        &mut self,
        container_creation_definition: &ContainerCreationDefintition,
    ) {
        let mut scene_graph = self.world.write_resource::<SceneGraph>();
        let mut container_mapping = self.world.write_resource::<ContainerMapping>();
        unimplemented!();
    }

    pub fn update_container(&mut self, container_update_definition: &ContainerUpdateDefintition) {
        let mut scene_graph = self.world.write_resource::<SceneGraph>();
        let mut container_mapping = self.world.read_resource::<ContainerMapping>();
        unimplemented!();
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

    pub fn get_drawable_items(&self) -> Vec<DrawableItem> {
        let library = self.get_library();
        let scene_graph = self.get_scene_graph();
        let display_storage = self.world.read_storage::<Display>();
        let transform_storage = self.world.read_storage::<Transform>();
        let coloring_storage = self.world.read_storage::<Coloring>();
        let view_rect_storage = self.world.read_storage::<ViewRect>();
        let order_storage = self.world.read_storage::<Order>();
        let morph_storage = self.world.read_storage::<Morph>();
        let mut unordered = (
            &self.world.entities(),
            &display_storage,
            &transform_storage,
            (&coloring_storage).maybe(),
            (&view_rect_storage).maybe(),
            (&order_storage).maybe(),
            (&morph_storage).maybe(),
        )
            .join()
            .filter_map(
                |(entity, display, transform, coloring, view_rect, order, morph)| {
                    if let Some(library_item) = match display.1 {
                        DisplayKind::Vector => library
                            .get_shape(&display.0)
                            .and_then(|shape| Some(LibraryItem::Vector(shape))),
                        DisplayKind::Raster => library
                            .get_texture(&display.0)
                            .and_then(|pattern| Some(LibraryItem::Raster(pattern))),
                    } {
                        Some((
                            entity,
                            (
                                order.copied().unwrap_or_default().0,
                                DrawableItem {
                                    library_item,
                                    transform: transform.world,
                                    coloring: coloring.cloned(),
                                    view_rect: view_rect.and_then(|view_rect| Some(view_rect.0)),
                                    morph: morph
                                        .and_then(|morph| Some(morph.0))
                                        .unwrap_or_default(),
                                },
                            ),
                        ))
                    } else {
                        None
                    }
                },
            )
            .collect::<HashMap<Entity, (i8, DrawableItem)>>();
        let mut sorted = vec![];
        let mut queue = VecDeque::new();
        queue.push_back(scene_graph.root());
        while let Some(next) = queue.pop_front() {
            let mut children = scene_graph.get_children(next).clone().unwrap();
            children.sort_by(|a, b| {
                let order_a = unordered
                    .get(&a)
                    .and_then(|(order, _)| Some(*order))
                    .unwrap_or_default();
                let order_b = unordered
                    .get(&b)
                    .and_then(|(order, _)| Some(*order))
                    .unwrap_or_default();
                order_a.cmp(&order_b)
            });
            for child in children {
                queue.push_back(child);
                if let Some((_, display_item)) = unordered.remove(child) {
                    sorted.push(display_item)
                };
            }
        }
        sorted
    }
}

#[derive(Debug)]
pub enum LibraryItem<'a> {
    Vector(&'a Shape),
    Raster(&'a Pattern),
}

#[derive(Debug)]
pub struct DrawableItem<'a> {
    pub library_item: LibraryItem<'a>,
    pub transform: Transform2F,
    pub coloring: Option<Coloring>,
    pub view_rect: Option<RectF>,
    pub morph: f32,
}
