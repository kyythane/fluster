use super::{
    components::{Bounds, Morph, Transform, Tweens},
    resources::{FrameTime, SceneGraph},
};
use crate::tween::{PropertyTweenData, PropertyTweenUpdate, Tween};
use specs::prelude::*;

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
    type SystemData = (WriteStorage<'a, Transform>, ReadExpect<'a, SceneGraph>);

    fn run(&mut self, (mut transform_storage, scene_graph): Self::SystemData) {
        // update tweens
    }
}

/*struct UpdateBounds;

impl<'a> System<'a> for UpdateBounds {
    type SystemData = (WriteStorage<'a, Bounds>);

    fn run(&mut self, (mut transform_storage): Self::SystemData) {
        // update tweens
    }
}*/

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
