use super::{
    components::{
        Bounds, BoundsSource, Display, DisplayKind, Layer, Morph, Order, Transform, Tweens,
        ViewRect,
    },
    resources::{
        ContainerCreationQueue, ContainerMapping, ContainerUpdateQueue, FrameTime, Library,
        QuadTreeLayer, QuadTrees, SceneGraph,
    },
};
use crate::{
    actions::{
        BoundsKindDefinition, ContainerCreationProperty, ContainerUpdateProperty, RectPoints,
    },
    tween::{PropertyTween, PropertyTweenData, PropertyTweenUpdate, Tween, TweenDuration},
    types::{basic::ScaleRotationTranslation, coloring::Coloring},
};
use palette::LinSrgba;
use pathfinder_geometry::{rect::RectF, transform2d::Transform2F, vector::Vector2F};
use reduce::Reduce;
use specs::{
    shred::ResourceId, Entities, Entity, Join, Read, ReadExpect, ReadStorage, System, SystemData,
    World, Write, WriteExpect, WriteStorage,
};
use std::collections::{HashSet, VecDeque};

#[derive(SystemData)]
pub struct ContainerCreationSystemData<'a> {
    container_mapping: Write<'a, ContainerMapping>,
    container_creation_queue: Write<'a, ContainerCreationQueue>,
    scene_graph: WriteExpect<'a, SceneGraph>,
    library: Read<'a, Library>,
    entities: Entities<'a>,
    transform_storage: WriteStorage<'a, Transform>,
    order_storage: WriteStorage<'a, Order>,
    morph_storage: WriteStorage<'a, Morph>,
    bounds_storage: WriteStorage<'a, Bounds>,
    layer_storage: WriteStorage<'a, Layer>,
    view_rect_storage: WriteStorage<'a, ViewRect>,
    coloring_storage: WriteStorage<'a, Coloring>,
    display_storage: WriteStorage<'a, Display>,
}

pub struct ContainerCreation;

impl<'a> System<'a> for ContainerCreation {
    type SystemData = ContainerCreationSystemData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        while let Some(definition) = data.container_creation_queue.dequeue() {
            if data.container_mapping.contains_container(definition.id()) {
                // TODO: errors
                todo!();
            } else if !data
                .container_mapping
                .contains_container(definition.parent())
            {
                // TODO: errors
                todo!();
            } else {
                let mut entity_builder = data.entities.build_entity();

                let parent_entity = *data
                    .container_mapping
                    .get_entity(definition.parent())
                    .unwrap();
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
                            entity_builder =
                                entity_builder.with(transform, &mut data.transform_storage);
                        }
                        ContainerCreationProperty::MorphIndex(morph) => {
                            entity_builder =
                                entity_builder.with(Morph(*morph), &mut data.morph_storage);
                        }
                        ContainerCreationProperty::Coloring(coloring) => {
                            entity_builder =
                                entity_builder.with(coloring.clone(), &mut data.coloring_storage);
                        }
                        ContainerCreationProperty::ViewRect(rect_points) => {
                            entity_builder = entity_builder.with(
                                ViewRect(RectF::from_points(
                                    rect_points.origin,
                                    rect_points.lower_right,
                                )),
                                &mut data.view_rect_storage,
                            );
                        }
                        ContainerCreationProperty::Display(display) => {
                            let display_item = if data.library.contains_shape(display) {
                                Display(*display, DisplayKind::Vector)
                            } else if data.library.contains_texture(display) {
                                Display(*display, DisplayKind::Raster)
                            } else {
                                // TODO: errors
                                panic!()
                            };
                            entity_builder =
                                entity_builder.with(display_item, &mut data.display_storage);
                        }
                        ContainerCreationProperty::Order(order) => {
                            entity_builder =
                                entity_builder.with(Order(*order), &mut data.order_storage);
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
                            entity_builder = entity_builder.with(bounds, &mut data.bounds_storage);
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
                    entity_builder =
                        entity_builder.with(Layer { quad_trees: layers }, &mut data.layer_storage);
                }
                let entity = entity_builder.build();
                data.container_mapping
                    .add_container(*definition.id(), entity);
                data.scene_graph.add_entity(&parent_entity, &entity);
                // NOTE: not inserting into quad tree since that will get handled during the first frame this entity exists in
            }
        }
    }
}
#[derive(SystemData)]
pub struct ContainerUpdateSystemData<'a> {
    container_mapping: Write<'a, ContainerMapping>,
    container_update_queue: Write<'a, ContainerUpdateQueue>,
    scene_graph: WriteExpect<'a, SceneGraph>,
    quad_trees: Write<'a, QuadTrees>,
    library: Read<'a, Library>,
    transform_storage: WriteStorage<'a, Transform>,
    order_storage: WriteStorage<'a, Order>,
    morph_storage: WriteStorage<'a, Morph>,
    bounds_storage: WriteStorage<'a, Bounds>,
    layer_storage: WriteStorage<'a, Layer>,
    view_rect_storage: WriteStorage<'a, ViewRect>,
    coloring_storage: WriteStorage<'a, Coloring>,
    display_storage: WriteStorage<'a, Display>,
    tween_storage: WriteStorage<'a, Tweens>,
}

pub struct ContainerUpdate;

impl ContainerUpdate {
    fn add_tween(tween_storage: &mut WriteStorage<Tweens>, entity: Entity, tween: PropertyTween) {
        tween_storage
            .entry(entity)
            .unwrap()
            .or_insert(Tweens(vec![]))
            .0
            .push(tween);
    }
}

impl<'a> System<'a> for ContainerUpdate {
    type SystemData = ContainerUpdateSystemData<'a>;
    fn run(&mut self, mut data: Self::SystemData) {
        while let Some(definition) = data.container_update_queue.dequeue() {
            if let Some(entity) = data.container_mapping.get_entity(definition.id()) {
                let entity = *entity;
                for property in definition.properties() {
                    match property {
                        ContainerUpdateProperty::Transform(srt, easing, duration_frames) => {
                            let start = data
                                .transform_storage
                                .entry(entity)
                                .unwrap()
                                .or_insert(Transform::default())
                                .local;
                            let tween = PropertyTween::new_transform(
                                ScaleRotationTranslation::from_transform(&start),
                                *srt,
                                TweenDuration::new_frame(*duration_frames),
                                *easing,
                            );
                            Self::add_tween(&mut data.tween_storage, entity, tween);
                        }
                        ContainerUpdateProperty::MorphIndex(morph, easing, duration_frames) => {
                            let start = data
                                .morph_storage
                                .entry(entity)
                                .unwrap()
                                .or_insert(Morph::default())
                                .0;
                            let tween = PropertyTween::new_morph_index(
                                start,
                                *morph,
                                TweenDuration::new_frame(*duration_frames),
                                *easing,
                            );
                            Self::add_tween(&mut data.tween_storage, entity, tween);
                        }
                        ContainerUpdateProperty::Coloring(
                            coloring,
                            color_space,
                            easing,
                            duration_frames,
                        ) => {
                            let library_item = data.display_storage.get(entity).and_then(
                                |display| match display.1 {
                                    DisplayKind::Vector => data
                                        .library
                                        .get_shape(&display.0)
                                        .and_then(|shape| Some(Some(shape.color())))
                                        .unwrap_or(None),
                                    DisplayKind::Raster => {
                                        Some(Coloring::Color(LinSrgba::new(1.0, 1.0, 1.0, 1.0)))
                                    }
                                },
                            );
                            let coloring_component = data.coloring_storage.get(entity);
                            let start = match (library_item, coloring_component) {
                                (_, Some(component)) => component.clone(),
                                (Some(coloring), None) => {
                                    data.coloring_storage
                                        .insert(entity, coloring.clone())
                                        .unwrap();
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
                            Self::add_tween(&mut data.tween_storage, entity, tween);
                        }
                        ContainerUpdateProperty::ViewRect(rect_points, easing, duration_frames) => {
                            let library_item = data
                                .display_storage
                                .get(entity)
                                .and_then(|display| data.library.get_texture(&display.0));
                            let view_rect_component = data.view_rect_storage.get(entity);
                            let start = match (library_item, view_rect_component) {
                                (_, Some(component)) => component.0,
                                (Some(pattern), None) => {
                                    let rect = RectF::from_points(
                                        Vector2F::zero(),
                                        pattern.size().to_f32(),
                                    );
                                    data.view_rect_storage
                                        .insert(entity, ViewRect(rect))
                                        .unwrap();
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
                            Self::add_tween(&mut data.tween_storage, entity, tween);
                        }
                        ContainerUpdateProperty::Order(order, easing, duration_frames) => {
                            let start = data
                                .order_storage
                                .entry(entity)
                                .unwrap()
                                .or_insert(Order::default())
                                .0;
                            let tween = PropertyTween::new_order(
                                start,
                                *order,
                                TweenDuration::new_frame(*duration_frames),
                                *easing,
                            );
                            Self::add_tween(&mut data.tween_storage, entity, tween);
                        }
                        ContainerUpdateProperty::Display(display) => {
                            let display_item = if data.library.contains_shape(display) {
                                Display(*display, DisplayKind::Vector)
                            } else if data.library.contains_texture(display) {
                                Display(*display, DisplayKind::Raster)
                            } else {
                                // TODO: errors
                                panic!()
                            };
                            data.display_storage.insert(entity, display_item).unwrap();
                        }
                        ContainerUpdateProperty::RemoveDisplay => {
                            data.display_storage.remove(entity);
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
                            data.bounds_storage.insert(entity, bounds).unwrap();
                        }
                        ContainerUpdateProperty::RemoveBounds => {
                            data.bounds_storage.remove(entity);
                        }
                        ContainerUpdateProperty::AddToLayer(layer) => {
                            if let Some(bounds) = data.bounds_storage.get_mut(entity) {
                                let layers = data
                                    .layer_storage
                                    .entry(entity)
                                    .unwrap()
                                    .or_insert(Layer::default());
                                if !layers.quad_trees.contains(layer) {
                                    layers.quad_trees.insert(*layer);
                                    bounds.dirty = true;
                                    // NOTE: not inserting into quad tree since that will get handled during the first frame after updating
                                }
                            }
                        }
                        ContainerUpdateProperty::RemoveFromLayer(layer) => {
                            if let Some(layers) = data.layer_storage.get_mut(entity) {
                                if layers.quad_trees.remove(layer) {
                                    data.quad_trees.remove_from_layer(layer, entity);
                                }
                            };
                        }
                        ContainerUpdateProperty::Parent(new_parent) => {
                            if let Some(new_parent) = data.container_mapping.get_entity(new_parent)
                            {
                                data.scene_graph.reparent(new_parent, entity);
                                if let Some(transfom) = data.transform_storage.get_mut(entity) {
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
        }
    }
}

pub struct ApplyTransformTweens;

impl<'a> System<'a> for ApplyTransformTweens {
    type SystemData = (WriteStorage<'a, Transform>, ReadStorage<'a, Tweens>);

    fn run(&mut self, (mut transform_storage, tweens_storage): Self::SystemData) {
        for (transform, tweens) in (&mut transform_storage, &tweens_storage).join() {
            let before_update = transform.local;
            tweens
                .0
                .iter()
                .filter_map(|tween| {
                    if let PropertyTweenData::Transform { .. } = tween.tween_data() {
                        if let PropertyTweenUpdate::Transform(end_transfom) = tween.compute() {
                            Some(end_transfom)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .reduce(|acc_transform, transform| transform * acc_transform)
                .map(|updated| {
                    if updated != before_update {
                        transform.dirty = true;
                    }
                    transform.local = updated;
                });
        }
    }
}

pub struct ApplyMorphTweens;

impl<'a> System<'a> for ApplyMorphTweens {
    type SystemData = (WriteStorage<'a, Morph>, ReadStorage<'a, Tweens>);

    fn run(&mut self, (mut morph_storage, tweens_storage): Self::SystemData) {
        for (morph, tweens) in (&mut morph_storage, &tweens_storage).join() {
            tweens
                .0
                .iter()
                .filter_map(|tween| {
                    if let PropertyTweenData::MorphIndex { .. } = tween.tween_data() {
                        if let PropertyTweenUpdate::Morph(end_morph) = tween.compute() {
                            Some(end_morph)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .reduce(|morph_acc, morph| morph_acc * morph)
                .map(|updated| {
                    morph.0 = updated;
                });
        }
    }
}

pub struct ApplyViewRectTweens;

impl<'a> System<'a> for ApplyViewRectTweens {
    type SystemData = (WriteStorage<'a, ViewRect>, ReadStorage<'a, Tweens>);

    fn run(&mut self, (mut view_storage, tweens_storage): Self::SystemData) {
        for (view_rect, tweens) in (&mut view_storage, &tweens_storage).join() {
            tweens
                .0
                .iter()
                .filter(|tween| {
                    if let PropertyTweenData::ViewRect { .. } = tween.tween_data() {
                        true
                    } else {
                        false
                    }
                })
                .map(|tween| {
                    if let PropertyTweenUpdate::ViewRect(end_rect) = tween.compute() {
                        end_rect
                    } else {
                        panic!();
                    }
                })
                .enumerate()
                .reduce(|(_, sum_rect), (index, rect)| {
                    (
                        index,
                        RectF::from_points(
                            sum_rect.origin() + rect.origin(),
                            sum_rect.lower_right() + rect.lower_right(),
                        ),
                    )
                })
                .map(|(count, sum_rect)| {
                    view_rect.0 = RectF::from_points(
                        sum_rect.origin() / (count + 1) as f32,
                        sum_rect.lower_right() / count as f32,
                    );
                });
        }
    }
}

pub struct ApplyColoringTweens;

impl<'a> System<'a> for ApplyColoringTweens {
    type SystemData = (WriteStorage<'a, Coloring>, ReadStorage<'a, Tweens>);

    fn run(&mut self, (mut coloring_storage, tweens_storage): Self::SystemData) {
        for (coloring, tweens) in (&mut coloring_storage, &tweens_storage).join() {
            tweens
                .0
                .iter()
                .filter(|tween| {
                    if let PropertyTweenData::Coloring { .. } = tween.tween_data() {
                        true
                    } else {
                        false
                    }
                })
                .map(|tween| {
                    if let PropertyTweenUpdate::Coloring(end_coloring) = tween.compute() {
                        end_coloring.into_denormalized()
                    } else {
                        panic!();
                    }
                })
                .enumerate()
                .reduce(|(_, sum_denormalized), (index, denormalized)| {
                    (index, sum_denormalized.add(&denormalized))
                })
                .map(|(count, sum_denormalized)| {
                    *coloring = (sum_denormalized.div((count + 1) as f32)).into_coloring();
                });
        }
    }
}

pub struct ApplyOrderTweens;

impl<'a> System<'a> for ApplyOrderTweens {
    type SystemData = (WriteStorage<'a, Order>, ReadStorage<'a, Tweens>);

    fn run(&mut self, (mut order_storage, tweens_storage): Self::SystemData) {
        for (order, tweens) in (&mut order_storage, &tweens_storage).join() {
            tweens
                .0
                .iter()
                .filter_map(|tween| {
                    if let PropertyTweenData::Order { .. } = tween.tween_data() {
                        if let PropertyTweenUpdate::Order(order) = tween.compute() {
                            Some(order)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .reduce(|max, order| max.max(order))
                .map(|updated| {
                    order.0 = updated;
                });
        }
    }
}

pub struct UpdateWorldTransform;

impl<'a> System<'a> for UpdateWorldTransform {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Transform>,
        ReadExpect<'a, SceneGraph>,
    );

    fn run(&mut self, (entities, mut transform_storage, scene_graph): Self::SystemData) {
        // First pass algorithm. O(m log n), where m is # dirty nodes and n is # total nodes.
        let dirty_roots = entities
            .join()
            .filter(|entity| {
                if let Some(transform) = transform_storage.get(*entity) {
                    transform.dirty
                } else {
                    false
                }
            })
            .map(|entity| {
                let mut maximal_id = &entity;
                for parent in scene_graph.get_parent_iter(&entity) {
                    if let Some(..) = transform_storage.get(*parent) {
                        maximal_id = parent;
                    }
                }
                *maximal_id
            })
            .collect::<HashSet<Entity>>();
        let mut queue = VecDeque::new();
        for dirty_root in dirty_roots {
            let mut current_world_transform = Transform2F::default();
            for parent in scene_graph.get_parent_iter(&dirty_root) {
                if let Some(transform) = transform_storage.get(*parent) {
                    current_world_transform = transform.world;
                    break;
                }
            }
            queue.push_back((dirty_root, current_world_transform));
            while let Some((next, current_world_transform)) = queue.pop_front() {
                let current_world_transform =
                    if let Some(transform) = transform_storage.get_mut(next) {
                        let transform_before_update = transform.world;
                        transform.world = current_world_transform * transform.local;
                        if transform.world != transform_before_update {
                            transform.dirty = true;
                        }
                        transform.world
                    } else {
                        current_world_transform
                    };
                for child in scene_graph.get_children(&next).unwrap() {
                    queue.push_back((*child, current_world_transform));
                }
            }
        }
    }
}

pub struct UpdateBounds;

impl<'a> System<'a> for UpdateBounds {
    type SystemData = (
        WriteStorage<'a, Bounds>,
        ReadStorage<'a, Transform>,
        ReadStorage<'a, Morph>,
        ReadStorage<'a, Display>,
        ReadStorage<'a, ViewRect>,
        Read<'a, Library>,
    );

    fn run(
        &mut self,
        (
            mut bounds_storage,
            transform_storage,
            morph_storage,
            display_storage,
            view_rect_storage,
            library,
        ): Self::SystemData,
    ) {
        for (bounds, transform, morph, display, view_rect) in (
            &mut bounds_storage,
            &transform_storage,
            (&morph_storage).maybe(),
            (&display_storage).maybe(),
            (&view_rect_storage).maybe(),
        )
            .join()
        {
            if transform.dirty {
                bounds.bounds = match bounds.source {
                    BoundsSource::Display => {
                        match display {
                            Some(Display(id, DisplayKind::Vector)) => {
                                let shape = library.get_shape(id).unwrap();
                                shape.compute_bounding(&transform.world, morph.unwrap_or(&Morph(0.0)).0)
                            }
                            Some(Display(id, DisplayKind::Raster)) => {
                                let pattern = library.get_texture(id).unwrap();
                                let (o, lr) = view_rect.and_then(|ViewRect(rect)| Some((rect.origin(), rect.lower_right()))).unwrap_or_else(|| (Vector2F::zero(), pattern.size().to_f32()));
                                let o = transform.world * o;
                                let lr = transform.world * lr;
                                RectF::from_points(o.min(lr), o.max(lr))
                            }
                            None => panic!("Attmpting to compute the bounds of an entity without an attachd display")
                        }
                    }
                    BoundsSource::Defined(rect) => {
                        let o = transform.world * rect.origin();
                        let lr = transform.world * rect.lower_right();
                        RectF::from_points(o.min(lr), o.max(lr))
                    }
                };
                bounds.dirty = true;
            }
        }
    }
}

pub struct UpdateQuadTree;

impl<'a> System<'a> for UpdateQuadTree {
    type SystemData = (
        Write<'a, QuadTrees>,
        Entities<'a>,
        ReadStorage<'a, Bounds>,
        ReadStorage<'a, Layer>,
    );

    fn run(&mut self, (mut quad_trees, entities, bounds_storage, layer_storage): Self::SystemData) {
        for (entity, bounds, layers) in (&*entities, &bounds_storage, &layer_storage).join() {
            if bounds.dirty {
                let aabb = bounds.bounds;
                layers
                    .quad_trees
                    .iter()
                    .for_each(|tree| quad_trees.update(entity, *tree, aabb));
            }
        }
    }
}

pub struct UpdateTweens;

impl<'a> System<'a> for UpdateTweens {
    type SystemData = (WriteStorage<'a, Tweens>, Read<'a, FrameTime>);

    fn run(&mut self, (mut tweens_storage, frame_time): Self::SystemData) {
        // update tweens
        (&mut tweens_storage).join().for_each(|tweens| {
            tweens
                .0
                .iter_mut()
                .for_each(|tween| tween.update(frame_time.delta_frame, frame_time.delta_time))
        });
        // filter out complete tweens
        for tweens in (&mut tweens_storage).join() {
            tweens.0.retain(|tween| !tween.is_complete());
        }
    }
}
