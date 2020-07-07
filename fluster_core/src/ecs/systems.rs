use super::{
    components::{Bounds, Morph, RasterDisplay, Transform, Tweens, VectorDisplay},
    resources::{FrameTime, SceneGraph},
};
use crate::tween::{PropertyTweenData, PropertyTweenUpdate, Tween};
use pathfinder_geometry::transform2d::Transform2F;
use specs::prelude::*;
use std::collections::{HashSet, VecDeque};

struct ApplyTransformTweens;

impl<'a> System<'a> for ApplyTransformTweens {
    type SystemData = (WriteStorage<'a, Transform>, ReadStorage<'a, Tweens>);

    fn run(&mut self, (mut transform_storage, tweens_storage): Self::SystemData) {
        for (transform, Tweens(tweens)) in (&mut transform_storage, &tweens_storage).join() {
            let before_update = transform.local;
            transform.local = tweens
                .iter()
                .filter(|tween| {
                    if let PropertyTweenData::Transform { .. } = tween.tween_data() {
                        true
                    } else {
                        false
                    }
                })
                .fold(transform.local, |transform, tween| {
                    if let PropertyTweenUpdate::Transform(end_transfom) = tween.compute() {
                        transform * end_transfom
                    } else {
                        transform
                    }
                });
            if transform.local != before_update {
                transform.touched = true;
            }
        }
    }
}

struct ApplyMorphTweens;

impl<'a> System<'a> for ApplyMorphTweens {
    type SystemData = (WriteStorage<'a, Morph>, ReadStorage<'a, Tweens>);

    fn run(&mut self, (mut morph_storage, tweens_storage): Self::SystemData) {
        for (morph, Tweens(tweens)) in (&mut morph_storage, &tweens_storage).join() {
            morph.0 = tweens
                .iter()
                .filter(|tween| {
                    if let PropertyTweenData::MorphIndex { .. } = tween.tween_data() {
                        true
                    } else {
                        false
                    }
                })
                .fold(morph.0, |morph, tween| {
                    if let PropertyTweenUpdate::Morph(end_morph) = tween.compute() {
                        morph * end_morph
                    } else {
                        morph
                    }
                });
        }
    }
}

struct UpdateWorldTransform;

impl<'a> System<'a> for UpdateWorldTransform {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Transform>,
        ReadExpect<'a, SceneGraph>,
    );

    fn run(&mut self, (entities, mut transform_storage, scene_graph): Self::SystemData) {
        let dirty_roots = entities
            .join()
            .filter(|entity| {
                if let Some(transform) = transform_storage.get(*entity) {
                    transform.touched
                } else {
                    false
                }
            })
            .map(|entity| {
                let mut maximal_id = &entity;
                for parent in scene_graph.get_parent_iter(&entity) {
                    if let Some(transform) = transform_storage.get(*parent) {
                        maximal_id = parent;
                    }
                }
                maximal_id
            })
            .collect::<HashSet<&Entity>>();
        let mut queue = VecDeque::new();
        for dirty_root in dirty_roots {
            let mut current_world_transform =
                if let Some(root_transform) = transform_storage.get(*dirty_root) {
                    let mut parent_transform = Transform2F::default();
                    for parent in scene_graph.get_parent_iter(dirty_root) {
                        if let Some(transform) = transform_storage.get(*parent) {
                            parent_transform = transform.world;
                            break;
                        }
                    }
                    parent_transform
                } else {
                    Transform2F::default()
                };
            queue.push_back(dirty_root);
            while let Some(next) = queue.pop_front() {
                for child in scene_graph.get_children(next).unwrap() {
                    queue.push_back(child);
                }
                if let Some(transform) = transform_storage.get_mut(*next) {
                    transform.world = current_world_transform * transform.local;
                    current_world_transform = transform.world;
                }
            }
        }
    }
}

struct UpdateBounds;

impl<'a> System<'a> for UpdateBounds {
    type SystemData = (
        WriteStorage<'a, Bounds>,
        ReadStorage<'a, VectorDisplay>,
        ReadStorage<'a, RasterDisplay>,
    );

    fn run(&mut self, (mut transform_storage, vector_storage, raster_storage): Self::SystemData) {
        // update tweens
    }
}

struct UpdateTweens;

impl<'a> System<'a> for UpdateTweens {
    type SystemData = (WriteStorage<'a, Tweens>, Read<'a, FrameTime>);

    fn run(&mut self, (mut tweens_storage, frame_time): Self::SystemData) {
        // update tweens
        (&mut tweens_storage)
            .join()
            .flat_map(|tweens| tweens.0)
            .for_each(|tween| tween.update(frame_time.delta_frame, frame_time.delta_time));
        // filter out complete tweens
        for tweens in (&mut tweens_storage).join() {
            tweens.0.retain(|tween| !tween.is_complete());
        }
    }
}
