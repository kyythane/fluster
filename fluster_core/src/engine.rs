use crate::{
    actions::{ContainerCreationDefintition, ContainerUpdateDefintition},
    ecs::{
        common::recompute_bounds,
        components::{
            Bounds, Display, DisplayKind, Layer, LocalTransform, Morph, Order, Tweens, ViewRect,
            WorldTransform,
        },
        resources::{
            ContainerCreationQueue, ContainerMapping, ContainerUpdateQueue, FrameTime, Library,
            QuadTreeQuery, QuadTrees, SceneGraph,
        },
        systems::{
            ApplyColoringTweens, ApplyMorphTweens, ApplyOrderTweens, ApplyTransformTweens,
            ApplyViewRectTweens, ContainerCreation, ContainerUpdate, UpdateBounds, UpdateQuadTree,
            UpdateTweens, UpdateWorldTransform,
        },
    },
    types::{
        basic::{ContainerId, LibraryId},
        coloring::Coloring,
        shapes::Shape,
    },
};
use pathfinder_content::pattern::Pattern;
use pathfinder_geometry::{rect::RectF, transform2d::Transform2F, vector::Vector2F};
use specs::{
    error::Error as SpecsError,
    shred::{Fetch, FetchMut},
    Builder, Dispatcher, DispatcherBuilder, Entity, Join, World, WorldExt,
};
use std::{
    cmp::Ordering,
    collections::{HashMap, VecDeque},
    sync::Arc,
};

pub struct Engine<'a, 'b> {
    root_container_id: ContainerId,
    world: World,
    dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> Engine<'a, 'b> {
    pub fn new(root_container_id: ContainerId, library: Library, quad_trees: QuadTrees) -> Self {
        let mut world = World::new();
        //Register components
        world.register::<LocalTransform>();
        world.register::<WorldTransform>();
        world.register::<Bounds>();
        world.register::<Morph>();
        world.register::<Order>();
        world.register::<Display>();
        world.register::<Tweens>();
        world.register::<Layer>();
        world.register::<Coloring>();
        world.register::<ViewRect>();

        // Setup resources
        let root = world
            .create_entity()
            .with(LocalTransform::default())
            .with(WorldTransform::default())
            .build();
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
                UpdateWorldTransform::default(),
                "update_world_transform",
                &["apply_transform_tweens"],
            )
            .with(
                UpdateBounds::default(),
                "update_bounds",
                &["update_world_transform", "apply_morph_tweens"],
            )
            .with(
                UpdateQuadTree::default(),
                "update_quad_tree",
                &["update_bounds"],
            )
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

    pub fn root_container_id(&self) -> &ContainerId {
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

    pub fn get_root_container_id(&self) -> ContainerId {
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

    pub fn remove_container(&mut self, container_id: &ContainerId) -> Result<(), SpecsError> {
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

    pub fn remove_container_and_children(
        &mut self,
        container_id: &ContainerId,
    ) -> Result<(), SpecsError> {
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

    pub fn refresh_bounds(&mut self, container_id: &ContainerId) {
        {
            let read_storage = self.world.read_resource::<ContainerMapping>();
            read_storage.get_entity(container_id).cloned()
        }
        .map(|entity| {
            let updated_bounds = {
                let bounds_storage = self.world.read_storage::<Bounds>();
                let transform_storage = self.world.read_storage::<WorldTransform>();
                let display_storage = self.world.read_storage::<Display>();
                let view_rect_storage = self.world.read_storage::<ViewRect>();
                let morph_storage = self.world.read_storage::<Morph>();
                let library = self.world.read_resource::<Library>();
                if let (Some(bounds), Some(transform)) =
                    (bounds_storage.get(entity), transform_storage.get(entity))
                {
                    Some(recompute_bounds(
                        &bounds.source,
                        transform.0,
                        display_storage.get(entity),
                        view_rect_storage.get(entity),
                        morph_storage.get(entity),
                        &*library,
                    ))
                } else {
                    None
                }
            };
            let mut bounds_storage = self.world.write_storage::<Bounds>();
            if let (Some(bounds_component), Some(updated_bounds)) =
                (bounds_storage.get_mut(entity), updated_bounds)
            {
                bounds_component.bounds = updated_bounds;
            }
        });
    }

    pub fn spatial_query(&self, query: &QuadTreeQuery) -> Vec<SelectionHandle> {
        let transform_storage = self.world.read_storage::<WorldTransform>();
        let morph_storage = self.world.read_storage::<Morph>();
        let display_storage = self.world.read_storage::<Display>();
        let container_mapping = self.world.read_resource::<ContainerMapping>();
        let library = self.world.read_resource::<Library>();
        self.world
            .read_resource::<QuadTrees>()
            .query(query)
            .map_or_else(
                || vec![],
                |entities| self.depth_sort_bounding_boxes(entities),
            )
            .into_iter()
            .map(|(entity, bounds)| {
                let container_id = container_mapping.get_container(&entity).copied().unwrap();
                let transform = transform_storage.get(entity).copied().unwrap_or_default().0;
                let morph = morph_storage.get(entity).copied().unwrap_or_default().0;
                let (shape_id, edge_list) = match display_storage.get(entity) {
                    Some(Display(shape_id, DisplayKind::Vector)) => library
                        .get_shape(shape_id)
                        .and_then(|shape| Some((Some(*shape_id), shape.edge_list(morph))))
                        .unwrap_or((Some(*shape_id), vec![])),
                    _ => (None, vec![]),
                };
                let handles = edge_list
                    .into_iter()
                    .enumerate()
                    .flat_map(|(index, edge)| match query {
                        QuadTreeQuery::Point(_, point) => edge
                            .query_disk(point, 10.0, &transform)
                            .map(|(vertex_index, distance, vertex)| {
                                VertexHandle::new(vertex, index, vertex_index, distance)
                            })
                            .collect::<Vec<VertexHandle>>(),
                        QuadTreeQuery::Disk(_, point, radius) => edge
                            .query_disk(point, *radius, &transform)
                            .map(|(vertex_index, distance, vertex)| {
                                VertexHandle::new(vertex, index, vertex_index, distance)
                            })
                            .collect::<Vec<VertexHandle>>(),
                        QuadTreeQuery::Rect(_, rect) => edge
                            .query_rect(rect, &transform)
                            .map(|(vertex_index, vertex)| {
                                VertexHandle::new(vertex, index, vertex_index, 0.0)
                            })
                            .collect::<Vec<VertexHandle>>(),
                        QuadTreeQuery::Ray(layer, origin, direction) => {
                            //Todo Not used yet and is a pain, so this is future Lily's problem
                            todo!()
                        }
                    })
                    .collect();
                SelectionHandle::new(container_id, shape_id, transform, bounds, morph, handles)
            })
            .collect()
    }

    pub fn depth_sort_bounding_boxes(
        &self,
        mut entities: Vec<(Entity, RectF)>,
    ) -> Vec<(Entity, RectF)> {
        let order_storage = self.world.read_storage::<Order>();
        let scene_graph = self.world.read_resource::<SceneGraph>();
        let orderings = entities
            .iter()
            .map(|(entity, _)| {
                (*entity, {
                    let mut ordering = scene_graph
                        .get_parent_iter(entity)
                        .map(|parent| order_storage.get(*parent).copied().unwrap_or_default().0)
                        .collect::<Vec<i8>>();
                    ordering.reverse();
                    ordering.push(order_storage.get(*entity).copied().unwrap_or_default().0);
                    ordering
                })
            })
            .collect::<HashMap<Entity, Vec<i8>>>();
        entities.sort_by(|(entity_a, _), (entity_b, _)| {
            for (order_a, order_b) in orderings
                .get(entity_a)
                .unwrap()
                .iter()
                .zip(orderings.get(entity_b).unwrap().iter())
            {
                // Sort front to back
                match order_b.cmp(order_a) {
                    Ordering::Less => return Ordering::Less,
                    Ordering::Greater => return Ordering::Greater,
                    _ => (),
                }
            }
            entity_a.cmp(entity_b)
        });
        entities
    }

    pub fn get_drawable_items(&self) -> Vec<DrawableItem> {
        let library = self.get_library();
        let scene_graph = self.get_scene_graph();
        let display_storage = self.world.read_storage::<Display>();
        let transform_storage = self.world.read_storage::<WorldTransform>();
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
                                    transform: transform.0,
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
            // Sort back to front
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

#[derive(Clone, Debug)]
pub struct SelectionHandle {
    container_id: ContainerId,
    shape_id: Option<LibraryId>,
    world_transform: Transform2F,
    bounds: RectF,
    morph: f32,
    handles: Vec<VertexHandle>,
}

impl SelectionHandle {
    pub fn new(
        container_id: ContainerId,
        shape_id: Option<LibraryId>,
        world_transform: Transform2F,
        bounds: RectF,
        morph: f32,
        handles: Vec<VertexHandle>,
    ) -> Self {
        Self {
            container_id,
            shape_id,
            world_transform,
            bounds,
            morph,
            handles,
        }
    }

    pub fn container_id(&self) -> &ContainerId {
        &self.container_id
    }

    pub fn shape_id(&self) -> &Option<LibraryId> {
        &self.shape_id
    }

    pub fn world_transform(&self) -> &Transform2F {
        &self.world_transform
    }

    pub fn bounds(&self) -> &RectF {
        &self.bounds
    }

    pub fn morph(&self) -> f32 {
        self.morph
    }

    pub fn handles(&self) -> &Vec<VertexHandle> {
        &self.handles
    }

    pub fn min_vertex(&self) -> Option<&VertexHandle> {
        self.handles
            .iter()
            .min_by(|a, b| a.separation.partial_cmp(&b.separation).unwrap())
    }
}

#[derive(Clone, Debug)]
pub struct VertexHandle {
    position: Vector2F,
    vertex_id: usize,
    edge_id: usize,
    separation: f32,
}

impl VertexHandle {
    pub fn new(position: Vector2F, edge_id: usize, vertex_id: usize, separation: f32) -> Self {
        Self {
            position,
            vertex_id,
            edge_id,
            separation,
        }
    }

    pub fn position(&self) -> Vector2F {
        self.position
    }

    pub fn edge_id(&self) -> usize {
        self.edge_id
    }

    pub fn vertex_id(&self) -> usize {
        self.vertex_id
    }

    pub fn separation(&self) -> f32 {
        self.separation
    }
}
