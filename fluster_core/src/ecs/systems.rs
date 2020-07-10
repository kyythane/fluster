use super::{
    components::{
        Bounds, BoundsSource, Coloring, Display, DisplayKind, Layer, Morph, Transform, Tweens,
        ViewRect,
    },
    resources::{FrameTime, Library, QuadTrees, SceneGraph},
};
use crate::tween::{PropertyTweenData, PropertyTweenUpdate, Tween};
use pathfinder_geometry::{rect::RectF, transform2d::Transform2F, vector::Vector2F};
use reduce::Reduce;
use specs::Join;
use specs::{Entities, Entity, Read, ReadExpect, ReadStorage, System, Write, WriteStorage};
use std::collections::{HashSet, VecDeque};

pub struct ApplyTransformTweens;

impl<'a> System<'a> for ApplyTransformTweens {
    type SystemData = (WriteStorage<'a, Transform>, ReadStorage<'a, Tweens>);

    fn run(&mut self, (mut transform_storage, tweens_storage): Self::SystemData) {
        for (transform, tweens) in (&mut transform_storage, &tweens_storage).join() {
            let before_update = transform.local;
            transform.local = tweens
                .0
                .iter()
                .filter(|tween| {
                    if let PropertyTweenData::Transform { .. } = tween.tween_data() {
                        true
                    } else {
                        false
                    }
                })
                .fold(Transform2F::default(), |transform, tween| {
                    if let PropertyTweenUpdate::Transform(end_transfom) = tween.compute() {
                        transform * end_transfom
                    } else {
                        transform
                    }
                });
            if transform.local != before_update {
                transform.dirty = true;
            }
        }
    }
}

pub struct ApplyMorphTweens;

impl<'a> System<'a> for ApplyMorphTweens {
    type SystemData = (WriteStorage<'a, Morph>, ReadStorage<'a, Tweens>);

    fn run(&mut self, (mut morph_storage, tweens_storage): Self::SystemData) {
        for (morph, tweens) in (&mut morph_storage, &tweens_storage).join() {
            morph.0 = tweens
                .0
                .iter()
                .filter(|tween| {
                    if let PropertyTweenData::MorphIndex { .. } = tween.tween_data() {
                        true
                    } else {
                        false
                    }
                })
                .fold(1.0, |morph, tween| {
                    if let PropertyTweenUpdate::Morph(end_morph) = tween.compute() {
                        morph * end_morph
                    } else {
                        morph
                    }
                });
        }
    }
}

pub struct ApplyViewRectTweens;

impl<'a> System<'a> for ApplyViewRectTweens {
    type SystemData = (WriteStorage<'a, ViewRect>, ReadStorage<'a, Tweens>);

    fn run(&mut self, (mut view_storage, tweens_storage): Self::SystemData) {
        for (view_rect, tweens) in (&mut view_storage, &tweens_storage).join() {
            let (count, sum_rect) = tweens
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
                        index + 1,
                        RectF::from_points(
                            sum_rect.origin() + rect.origin(),
                            sum_rect.lower_right() + rect.lower_right(),
                        ),
                    )
                })
                .unwrap_or_else(|| (1, view_rect.0));
            view_rect.0 = RectF::from_points(
                sum_rect.origin() / count as f32,
                sum_rect.lower_right() / count as f32,
            );
        }
    }
}

pub struct ApplyColoringTweens;

impl<'a> System<'a> for ApplyColoringTweens {
    type SystemData = (WriteStorage<'a, Coloring>, ReadStorage<'a, Tweens>);

    fn run(&mut self, (mut coloring_storage, tweens_storage): Self::SystemData) {}
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
            queue.push_back(dirty_root);
            while let Some(next) = queue.pop_front() {
                for child in scene_graph.get_children(&next).unwrap() {
                    queue.push_back(*child);
                }
                if let Some(transform) = transform_storage.get_mut(next) {
                    let transform_before_update = transform.world;
                    transform.world = current_world_transform * transform.local;
                    if transform.world != transform_before_update {
                        transform.dirty = true;
                    }
                    current_world_transform = transform.world;
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
                            Some(Display(uuid, DisplayKind::Vector)) => {
                                let shape = library.get_shape(uuid).unwrap();
                                shape.compute_bounding(&transform.world, morph.unwrap_or(&Morph(0.0)).0)
                            }
                            Some(Display(uuid, DisplayKind::Raster)) => {
                                let pattern = library.get_texture(uuid).unwrap();
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
