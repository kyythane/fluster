use crate::{
    actions::{
        BoundsKindDefinition, ContainerCreationDefintition, ContainerCreationProperty,
        ContainerUpdateDefintition, ContainerUpdateProperty, RectPoints,
    },
    ecs::{
        components::{
            Bounds, BoundsSource, Display, DisplayKind, Layer, Morph, Order, Transform, Tweens,
            ViewRect,
        },
        resources::{ContainerMapping, FrameTime, Library, QuadTreeLayer, QuadTrees, SceneGraph},
        systems::{
            ApplyColoringTweens, ApplyMorphTweens, ApplyOrderTweens, ApplyTransformTweens,
            ApplyViewRectTweens, UpdateBounds, UpdateQuadTree, UpdateTweens, UpdateWorldTransform,
        },
    },
    tween::{PropertyTween, TweenDuration},
    types::{basic::ScaleRotationTranslation, coloring::Coloring, shapes::Shape},
};
use palette::LinSrgba;
use pathfinder_content::pattern::Pattern;
use pathfinder_geometry::{rect::RectF, transform2d::Transform2F, vector::Vector2F};
use specs::{
    error::Error as SpecsError,
    shred::{Fetch, FetchMut},
    Builder, Dispatcher, DispatcherBuilder, Entity, Join, World, WorldExt,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};
use uuid::Uuid;

pub struct Engine<'a, 'b> {
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

        // Setup systems
        let dispatcher = DispatcherBuilder::new()
            .with(ApplyTransformTweens, "apply_transform_tweens", &[])
            .with(ApplyMorphTweens, "apply_morph_tweens", &[])
            .with(ApplyViewRectTweens, "apply_view_rect_tweens", &[])
            .with(ApplyColoringTweens, "apply_coloring_tweens", &[])
            .with(ApplyOrderTweens, "apply_order_tweens", &[])
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
        let container_mapping = self.world.read_resource::<ContainerMapping>();
        if container_mapping.contains_container(definition.id()) {
            // TODO: errors
            todo!();
        } else if !container_mapping.contains_container(definition.parent()) {
            // TODO: errors
            todo!();
        } else {
            let entities = self.world.entities_mut();
            let mut entity_builder = entities.build_entity();
            let parent_entity = *container_mapping.get_entity(definition.parent()).unwrap();
            for property in definition.properties() {
                match property {
                    ContainerCreationProperty::Transform(srt) => {
                        let transform = Transform {
                            local: Transform2F::from_scale_rotation_translation(
                                srt.scale,
                                srt.theta,
                                srt.translation,
                            ),
                            // NOTE: not definining world transform since that will get computed in first frame after entity is added
                            world: Transform2F::default(),
                            dirty: true,
                        };
                        entity_builder = entity_builder
                            .with(transform, &mut self.world.write_storage::<Transform>());
                    }
                    ContainerCreationProperty::MorphIndex(morph) => {
                        entity_builder = entity_builder
                            .with(Morph(*morph), &mut self.world.write_storage::<Morph>());
                    }
                    ContainerCreationProperty::Coloring(coloring) => {
                        entity_builder = entity_builder.with(
                            coloring.clone(),
                            &mut self.world.write_storage::<Coloring>(),
                        );
                    }
                    ContainerCreationProperty::ViewRect(rect_points) => {
                        entity_builder = entity_builder.with(
                            ViewRect(RectF::from_points(
                                rect_points.origin,
                                rect_points.lower_right,
                            )),
                            &mut self.world.write_storage::<ViewRect>(),
                        );
                    }
                    ContainerCreationProperty::Display(display) => {
                        let library = self.world.read_resource::<Library>();
                        let display_item = if library.contains_shape(display) {
                            Display(*display, DisplayKind::Vector)
                        } else if library.contains_texture(display) {
                            Display(*display, DisplayKind::Raster)
                        } else {
                            // TODO: errors
                            panic!()
                        };
                        entity_builder = entity_builder
                            .with(display_item, &mut self.world.write_storage::<Display>());
                    }
                    ContainerCreationProperty::Order(order) => {
                        entity_builder = entity_builder
                            .with(Order(*order), &mut self.world.write_storage::<Order>());
                    }
                    ContainerCreationProperty::Bounds(bounds_definition) => {
                        let bounds = match bounds_definition {
                            BoundsKindDefinition::Display => Bounds {
                                bounds: RectF::default(),
                                source: BoundsSource::Display,
                                dirty: true,
                            },
                            BoundsKindDefinition::Defined(rect_points) => Bounds {
                                // NOTE: not definining bounds since that will get computed in first frame after entity is added
                                bounds: RectF::default(),
                                source: BoundsSource::Defined(RectF::from_points(
                                    rect_points.origin,
                                    rect_points.lower_right,
                                )),
                                dirty: true,
                            },
                        };
                        entity_builder =
                            entity_builder.with(bounds, &mut self.world.write_storage::<Bounds>());
                    }
                    ContainerCreationProperty::Layer(..) => {}
                }
            }
            // Layers work a little differently since there could be multiple provided
            let layers = definition
                .properties()
                .iter()
                .filter_map(|property| {
                    if let ContainerCreationProperty::Layer(layer) = property {
                        Some(*layer)
                    } else {
                        None
                    }
                })
                .collect::<HashSet<QuadTreeLayer>>();
            if layers.len() > 0 {
                entity_builder = entity_builder.with(
                    Layer { quad_trees: layers },
                    &mut self.world.write_storage::<Layer>(),
                );
            }
            let entity = entity_builder.build();
            let mut container_mapping = self.world.write_resource::<ContainerMapping>();
            container_mapping.add_container(*definition.id(), entity);
            let mut scene_graph = self.world.write_resource::<SceneGraph>();
            scene_graph.add_entity(&parent_entity, &entity);
            // NOTE: not inserting into quad tree since that will get handled during the first frame this entity exists in
        }
    }

    pub fn update_container(
        &mut self,
        definition: &ContainerUpdateDefintition,
    ) -> Result<(), SpecsError> {
        let container_mapping = self.world.read_resource::<ContainerMapping>();
        if let Some(entity) = container_mapping.get_entity(definition.id()) {
            let entity = *entity;
            for property in definition.properties() {
                match property {
                    ContainerUpdateProperty::Transform(srt, easing, duration_frames) => {
                        let start = self
                            .world
                            .write_storage::<Transform>()
                            .entry(entity)?
                            .or_insert(Transform::default())
                            .local;
                        let tween = PropertyTween::new_transform(
                            ScaleRotationTranslation::from_transform(&start),
                            *srt,
                            TweenDuration::new_frame(*duration_frames),
                            *easing,
                        );
                        self.world
                            .write_storage::<Tweens>()
                            .entry(entity)?
                            .or_insert(Tweens(vec![]))
                            .0
                            .push(tween);
                    }
                    ContainerUpdateProperty::MorphIndex(morph, easing, duration_frames) => {
                        let start = self
                            .world
                            .write_storage::<Morph>()
                            .entry(entity)?
                            .or_insert(Morph::default())
                            .0;
                        let tween = PropertyTween::new_morph_index(
                            start,
                            *morph,
                            TweenDuration::new_frame(*duration_frames),
                            *easing,
                        );
                        self.world
                            .write_storage::<Tweens>()
                            .entry(entity)?
                            .or_insert(Tweens(vec![]))
                            .0
                            .push(tween);
                    }
                    ContainerUpdateProperty::Coloring(
                        coloring,
                        color_space,
                        easing,
                        duration_frames,
                    ) => {
                        let library = self.world.read_resource::<Library>();
                        let library_item = self
                            .world
                            .read_storage::<Display>()
                            .get(entity)
                            .and_then(|display| match display.1 {
                                DisplayKind::Vector => library
                                    .get_shape(&display.0)
                                    .and_then(|shape| Some(Some(shape.color())))
                                    .unwrap_or(None),
                                DisplayKind::Raster => {
                                    Some(Coloring::Color(LinSrgba::new(1.0, 1.0, 1.0, 1.0)))
                                }
                            });
                        let mut coloring_stotage = self.world.write_storage::<Coloring>();
                        let coloring_component = coloring_stotage.get(entity);
                        let start = match (library_item, coloring_component) {
                            (_, Some(component)) => component.clone(),
                            (Some(coloring), None) => {
                                coloring_stotage.insert(entity, coloring.clone())?;
                                coloring
                            }
                            (_, _) => {
                                //TODO: errors
                                todo!()
                            }
                        };
                        let tween = PropertyTween::new_coloring(
                            start,
                            coloring.clone(),
                            *color_space,
                            TweenDuration::new_frame(*duration_frames),
                            *easing,
                        );
                        self.world
                            .write_storage::<Tweens>()
                            .entry(entity)?
                            .or_insert(Tweens(vec![]))
                            .0
                            .push(tween);
                    }
                    ContainerUpdateProperty::ViewRect(rect_points, easing, duration_frames) => {
                        let library = self.world.read_resource::<Library>();
                        let library_item = self
                            .world
                            .read_storage::<Display>()
                            .get(entity)
                            .and_then(|display| library.get_texture(&display.0));
                        let mut view_rect_storage = self.world.write_storage::<ViewRect>();
                        let view_rect_component = view_rect_storage.get(entity);
                        let start = match (library_item, view_rect_component) {
                            (_, Some(component)) => component.0,
                            (Some(pattern), None) => {
                                let rect =
                                    RectF::from_points(Vector2F::zero(), pattern.size().to_f32());
                                view_rect_storage.insert(entity, ViewRect(rect))?;
                                rect
                            }
                            (_, _) => {
                                //TODO: errors
                                todo!()
                            }
                        };
                        let tween = PropertyTween::new_view_rect(
                            RectPoints::from_rect(&start),
                            *rect_points,
                            TweenDuration::new_frame(*duration_frames),
                            *easing,
                        );
                        self.world
                            .write_storage::<Tweens>()
                            .entry(entity)?
                            .or_insert(Tweens(vec![]))
                            .0
                            .push(tween);
                    }
                    ContainerUpdateProperty::Order(order, easing, duration_frames) => {
                        let start = self
                            .world
                            .write_storage::<Order>()
                            .entry(entity)?
                            .or_insert(Order::default())
                            .0;
                        let tween = PropertyTween::new_order(
                            start,
                            *order,
                            TweenDuration::new_frame(*duration_frames),
                            *easing,
                        );
                        self.world
                            .write_storage::<Tweens>()
                            .entry(entity)?
                            .or_insert(Tweens(vec![]))
                            .0
                            .push(tween);
                    }
                    ContainerUpdateProperty::Display(display) => {
                        let library = self.world.read_resource::<Library>();
                        let display_item = if library.contains_shape(display) {
                            Display(*display, DisplayKind::Vector)
                        } else if library.contains_texture(display) {
                            Display(*display, DisplayKind::Raster)
                        } else {
                            // TODO: errors
                            panic!()
                        };
                        self.world
                            .write_storage::<Display>()
                            .insert(entity, display_item)?;
                    }
                    ContainerUpdateProperty::RemoveDisplay => {
                        self.world.write_storage::<Display>().remove(entity);
                    }
                    ContainerUpdateProperty::Bounds(bounds_definition) => {
                        let bounds = match bounds_definition {
                            BoundsKindDefinition::Display => Bounds {
                                bounds: RectF::default(),
                                source: BoundsSource::Display,
                                dirty: true,
                            },
                            BoundsKindDefinition::Defined(rect_points) => Bounds {
                                // NOTE: not definining bounds since that will get computed in first frame after entity is updated
                                bounds: RectF::default(),
                                source: BoundsSource::Defined(RectF::from_points(
                                    rect_points.origin,
                                    rect_points.lower_right,
                                )),
                                dirty: true,
                            },
                        };
                        self.world
                            .write_storage::<Bounds>()
                            .insert(entity, bounds)?;
                    }
                    ContainerUpdateProperty::RemoveBounds => {
                        self.world.write_storage::<Bounds>().remove(entity);
                    }
                    ContainerUpdateProperty::AddToLayer(layer) => {}
                    ContainerUpdateProperty::RemoveFromLayer(layer) => {}
                    ContainerUpdateProperty::Parent(new_parent) => {
                        let mut scene_graph = self.world.write_resource::<SceneGraph>();
                        if let Some(new_parent) = container_mapping.get_entity(new_parent) {
                            scene_graph.reparent(new_parent, entity);
                            if let Some(transfom) =
                                self.world.write_storage::<Transform>().get_mut(entity)
                            {
                                transfom.dirty = true;
                            }
                        } else {
                            // Todo: errors
                            todo!();
                        }
                    }
                }
            }
        } else {
            //TODO: errors
            todo!();
        }
        Ok(())
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
