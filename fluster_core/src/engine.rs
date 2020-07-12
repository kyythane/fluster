use crate::{
    actions::{ContainerCreationDefintition, ContainerUpdateDefintition},
    ecs::{
        components::{
            Bounds, Display, DisplayKind, Layer, Morph, Order, Transform, Tweens, ViewRect,
        },
        resources::{
            ContainerCreationQueue, ContainerMapping, ContainerUpdateQueue, FrameTime, Library,
            QuadTrees, SceneGraph,
        },
        systems::{
            ApplyColoringTweens, ApplyMorphTweens, ApplyOrderTweens, ApplyTransformTweens,
            ApplyViewRectTweens, ContainerCreation, ContainerUpdate, UpdateBounds, UpdateQuadTree,
            UpdateTweens, UpdateWorldTransform,
        },
    },
    types::{coloring::Coloring, shapes::Shape},
};
use pathfinder_content::pattern::Pattern;
use pathfinder_geometry::{rect::RectF, transform2d::Transform2F};
use specs::{
    error::Error as SpecsError,
    shred::{Fetch, FetchMut},
    Builder, Dispatcher, DispatcherBuilder, Entity, Join, World, WorldExt,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use uuid::Uuid;

pub struct Engine<'a, 'b> {
    root_container_id: Uuid,
    world: World,
    dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> Engine<'a, 'b> {
    pub fn new(root_container_id: Uuid, library: Library, quad_trees: QuadTrees) -> Self {
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
        world.insert(quad_trees);
        let mut container_mapping = ContainerMapping::default();
        container_mapping.add_container(root_container_id, root);
        world.insert(container_mapping);
        world.insert(library);
        world.insert(ContainerCreationQueue::default());
        world.insert(ContainerUpdateQueue::default());

        // Setup systems
        let mut dispatcher = DispatcherBuilder::new()
            .with(ContainerCreation, "container_creation", &[])
            .with(ContainerUpdate, "container_update", &["container_creation"])
            .with(
                ApplyTransformTweens,
                "apply_transform_tweens",
                &["container_creation", "container_update"],
            )
            .with(
                ApplyMorphTweens,
                "apply_morph_tweens",
                &["container_creation", "container_update"],
            )
            .with(
                ApplyViewRectTweens,
                "apply_view_rect_tweens",
                &["container_creation", "container_update"],
            )
            .with(
                ApplyColoringTweens,
                "apply_coloring_tweens",
                &["container_creation", "container_update"],
            )
            .with(
                ApplyOrderTweens,
                "apply_order_tweens",
                &["container_creation", "container_update"],
            )
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
        dispatcher.setup(&mut world);
        Engine {
            root_container_id,
            world,
            dispatcher,
        }
    }

    pub fn update(&mut self, frame_time: FrameTime) {
        self.world.insert(frame_time);
        self.dispatcher.dispatch(&mut self.world);
        self.world.maintain();
    }

    pub fn root_container_id(&self) -> &Uuid {
        &self.root_container_id
    }

    pub fn get_scene_graph(&self) -> Fetch<SceneGraph> {
        self.world.read_resource::<SceneGraph>()
    }

    pub fn get_scene_graph_mut(&mut self) -> FetchMut<SceneGraph> {
        self.world.write_resource::<SceneGraph>()
    }

    pub fn get_container_mapping(&self) -> Fetch<ContainerMapping> {
        self.world.read_resource::<ContainerMapping>()
    }

    pub fn get_container_mapping_mut(&mut self) -> FetchMut<ContainerMapping> {
        self.world.write_resource::<ContainerMapping>()
    }

    pub fn get_library(&self) -> Fetch<Library> {
        self.world.read_resource::<Library>()
    }

    pub fn get_library_mut(&mut self) -> FetchMut<Library> {
        self.world.write_resource::<Library>()
    }

    pub fn get_quad_trees(&self) -> Fetch<QuadTrees> {
        self.world.read_resource::<QuadTrees>()
    }

    pub fn get_quad_trees_mut(&mut self) -> FetchMut<QuadTrees> {
        self.world.write_resource::<QuadTrees>()
    }

    pub fn get_root_container_id(&self) -> Uuid {
        let scene_graph = self.get_scene_graph();
        let container_mapping = self.get_container_mapping();
        *container_mapping.get_container(scene_graph.root()).unwrap()
    }

    pub fn create_container(&mut self, definition: &ContainerCreationDefintition) {
        let mut container_creation_queue = self.world.write_resource::<ContainerCreationQueue>();
        container_creation_queue.enqueue(definition.clone());
    }

    pub fn update_container(&mut self, definition: &ContainerUpdateDefintition) {
        let mut container_update_queue = self.world.write_resource::<ContainerUpdateQueue>();
        container_update_queue.enqueue(definition.clone());
    }

    pub fn remove_container(&mut self, container_id: &Uuid) -> Result<(), SpecsError> {
        let mut scene_graph = self.world.write_resource::<SceneGraph>();
        let mut container_mapping = self.world.write_resource::<ContainerMapping>();
        let mut quad_trees = self.world.write_resource::<QuadTrees>();
        let entities = self.world.entities_mut();
        let entity = container_mapping.get_entity(container_id).copied();
        if let Some(entity) = entity {
            container_mapping.remove_container(container_id);
            scene_graph.remove_entity(&entity);
            quad_trees.remove_all_layers(entity);
            entities.delete(entity)?;
        }
        Ok(())
    }

    pub fn remove_container_and_children(&mut self, container_id: &Uuid) -> Result<(), SpecsError> {
        let mut scene_graph = self.world.write_resource::<SceneGraph>();
        let mut container_mapping = self.world.write_resource::<ContainerMapping>();
        let entities = self.world.entities_mut();
        let mut quad_trees = self.world.write_resource::<QuadTrees>();
        if let Some(entity) = container_mapping.get_entity(container_id) {
            for entity in scene_graph.remove_entity_and_children(entity).into_iter() {
                container_mapping.remove_entity(&entity);
                quad_trees.remove_all_layers(entity);
                entities.delete(entity)?;
            }
        }
        Ok(())
    }

    pub fn mark_dirty(&mut self, container_id: &Uuid) {
        {
            let read_storage = self.world.read_resource::<ContainerMapping>();
            read_storage.get_entity(container_id).cloned()
        }
        .map(|entity| {
            let mut transform_storage = self.world.write_storage::<Transform>();
            transform_storage
                .get_mut(entity)
                .map(|transform| transform.dirty = true);
            let mut bounds_storage = self.world.write_storage::<Bounds>();
            bounds_storage
                .get_mut(entity)
                .map(|bounds| bounds.dirty = true);
        });
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
        queue.push_back(*scene_graph.root());
        while let Some(next) = queue.pop_front() {
            let mut children = scene_graph.get_children(&next).cloned().unwrap();
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
                if let Some((_, display_item)) = unordered.remove(&child) {
                    sorted.push(display_item)
                };
            }
        }
        sorted
    }
}

#[derive(Debug)]
pub enum LibraryItem {
    Vector(Arc<Shape>),
    Raster(Arc<Pattern>),
}

#[derive(Debug)]
pub struct DrawableItem {
    pub library_item: LibraryItem,
    pub transform: Transform2F,
    pub coloring: Option<Coloring>,
    pub view_rect: Option<RectF>,
    pub morph: f32,
}
