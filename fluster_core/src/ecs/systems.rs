use super::{
    components::{Morph, Tweens},
    resources::FrameTime,
};
use crate::tween::{PropertyTweenData, PropertyTweenUpdate, Tween};
use pathfinder_geometry::transform2d::Transform2F;
use specs::{prelude::*, storage::VecStorage, Component};
#[derive(Component, Debug)]
#[storage(VecStorage)]
struct Transform {
    local: Transform2F,
    world: Transform2F,
    touched: bool,
}

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
