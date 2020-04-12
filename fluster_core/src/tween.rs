#![deny(clippy::all)]

use super::actions::RectPoints;
use super::types::{basic::ScaleRotationTranslation, shapes::Coloring};
use super::util;
use pathfinder_color::{ColorF, ColorU};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use serde::{Deserialize, Serialize};
use std::f32::consts::{FRAC_PI_2, PI};
use std::f32::EPSILON;

pub trait Tween {
    type Item: ?Sized;

    fn update(&mut self, delta_time: f32) -> Self::Item;
    fn is_complete(&self) -> bool;
    fn easing(&self) -> Easing;
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Easing {
    Linear,
    QuadraticIn,
    QuadraticOut,
    QuadraticInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    QuarticIn,
    QuarticOut,
    QuarticInOut,
    QuinticIn,
    QuinticOut,
    QuinticInOut,
    SinusoidalIn,
    SinusoidalOut,
    SinusoidalInOut,
    ExponentialIn,
    ExponentialOut,
    ExponentialInOut,
    CircularIn,
    CircularOut,
    CircularInOut,
    ElasticIn,
    ElasticOut,
    ElasticInOut,
    BackIn,
    BackOut,
    BackInOut,
    BounceIn,
    BounceOut,
    BounceInOut,
    Step(u16),
    None,
}

//TODO: BUG: Back and Elastic are just wrong
impl Easing {
    pub fn ease(self, percent: f32) -> f32 {
        let percent = util::clamp_0_1(percent);
        const BACK_SCALE: f32 = 1.70158;
        const IN_OUT_BACK_SCALE: f32 = BACK_SCALE * 1.525;
        match self {
            Easing::Linear => percent,
            Easing::QuadraticIn => percent * percent,
            Easing::QuadraticOut => percent * (2.0 - percent),
            Easing::QuadraticInOut => {
                let percent = percent * 2.0;
                if percent < 1.0 {
                    percent * percent * 0.5
                } else {
                    let percent = percent - 1.0;
                    (1.0 - (percent * (percent - 2.0))) * 0.5
                }
            }
            Easing::CubicIn => percent * percent * percent,
            Easing::CubicOut => {
                let percent = percent - 1.0;
                percent * percent * percent + 1.0
            }
            Easing::CubicInOut => {
                let percent = percent * 2.0;
                if percent < 1.0 {
                    percent * percent * percent * 0.5
                } else {
                    let percent = percent - 2.0;
                    (percent * percent * percent + 2.0) * 0.5
                }
            }
            Easing::QuarticIn => percent * percent * percent * percent,
            Easing::QuarticOut => {
                let percent = percent - 1.0;
                1.0 - percent * percent * percent * percent
            }
            Easing::QuarticInOut => {
                let percent = percent * 2.0;
                if percent < 1.0 {
                    percent * percent * percent * percent * 0.5
                } else {
                    let percent = percent - 2.0;
                    (2.0 - percent * percent * percent * percent) * 0.5
                }
            }
            Easing::QuinticIn => percent * percent * percent * percent * percent,
            Easing::QuinticOut => {
                let percent = percent - 1.0;
                percent * percent * percent * percent * percent + 1.0
            }
            Easing::QuinticInOut => {
                let percent = percent * 2.0;
                if percent < 1.0 {
                    percent * percent * percent * percent * percent * 0.5
                } else {
                    let percent = percent - 2.0;
                    (percent * percent * percent * percent * percent + 2.0) * 0.5
                }
            }
            Easing::SinusoidalIn => 1.0 - (percent * FRAC_PI_2).cos(),
            Easing::SinusoidalOut => (percent * FRAC_PI_2).sin(),
            Easing::SinusoidalInOut => 0.5 * (1.0 - (percent * PI).cos()),
            Easing::ExponentialIn => {
                if percent.abs() > EPSILON {
                    2f32.powf((percent - 1.0) * 10.0)
                } else {
                    0.0
                }
            }
            Easing::ExponentialOut => {
                if percent.abs() > EPSILON {
                    1.0 - 2f32.powf(percent * -10.0)
                } else {
                    0.0
                }
            }
            Easing::ExponentialInOut => {
                if percent.abs() < EPSILON {
                    return 0.0;
                } else if (1.0 - percent).abs() < EPSILON {
                    return 1.0;
                }
                let percent = percent * 2.0 - 1.0;
                if percent < 1.0 {
                    2f32.powf(percent * 10.0) * 0.5
                } else {
                    (2.0 - 2f32.powf(percent * -10.0)) * 0.5
                }
            }
            Easing::CircularIn => (1.0 - percent * percent).sqrt(),
            Easing::CircularOut => {
                let percent = percent - 1.0;
                (1.0 - percent * percent).sqrt()
            }
            Easing::CircularInOut => {
                let percent = percent * 2.0;
                if percent < 1.0 {
                    (1.0 - (1.0 - percent * percent).sqrt()) * 0.5
                } else {
                    let percent = percent - 2.0;
                    (1.0 - percent * percent).sqrt() * 0.5
                }
            }
            Easing::ElasticIn => {
                if percent.abs() < EPSILON {
                    return 0.0;
                } else if (1.0 - percent).abs() < EPSILON {
                    return 1.0;
                }
                2f32.powf((percent - 1.0) * 10.0) * ((percent - 1.1) * 5.0 * PI).sin() * -1.0
            }
            Easing::ElasticOut => {
                if percent.abs() < EPSILON {
                    return 0.0;
                } else if (1.0 - percent).abs() < EPSILON {
                    return 1.0;
                }
                2f32.powf(percent * 10.0) * ((percent - 0.1) * 5.0 * PI).sin() + 1.0
            }
            Easing::ElasticInOut => {
                if percent.abs() < EPSILON {
                    return 0.0;
                } else if (1.0 - percent).abs() < EPSILON {
                    return 1.0;
                }
                let percent = percent * 2.0;
                if percent < 1.0 {
                    2f32.powf((percent - 1.0) * 10.0) * ((percent - 1.1) * 5.0 * PI).sin() * -0.5
                } else {
                    2f32.powf((percent - 1.0) * 10.0) * ((percent - 1.1) * 5.0 * PI).sin() * 0.5
                        + 1.0
                }
            }
            Easing::BackIn => percent * percent * ((BACK_SCALE + 1.0) * percent - BACK_SCALE),
            Easing::BackOut => {
                let percent = percent - 1.0;
                percent * percent * ((BACK_SCALE + 1.0) * percent - BACK_SCALE) + 1.0
            }
            Easing::BackInOut => {
                let percent = percent * 2.0;
                if percent < 1.0 {
                    percent
                        * percent
                        * ((IN_OUT_BACK_SCALE + 1.0) * percent - IN_OUT_BACK_SCALE)
                        * 0.5
                } else {
                    let percent = percent - 2.0;
                    (percent * percent * ((IN_OUT_BACK_SCALE + 1.0) * percent - IN_OUT_BACK_SCALE)
                        + 1.0)
                        * 0.5
                }
            }
            Easing::BounceIn => 1.0 - Easing::BounceOut.ease(1.0 - percent),
            Easing::BounceOut => {
                const RATIO: f32 = 7.5625;
                const LENGTH: f32 = 2.75;
                if percent < 1.0 / LENGTH {
                    RATIO * percent * percent
                } else if percent < 2.0 / LENGTH {
                    let percent = percent - 1.5 / LENGTH;
                    RATIO * percent * percent + 0.75
                } else if percent < 2.5 / LENGTH {
                    let percent = percent - 2.25 / LENGTH;
                    RATIO * percent * percent + 0.9375
                } else {
                    let percent = percent - 2.625 / LENGTH;
                    RATIO * percent * percent + 0.984_375
                }
            }
            Easing::BounceInOut => {
                if percent < 0.5 {
                    Easing::BounceIn.ease(percent * 2.0) * 0.5
                } else {
                    Easing::BounceOut.ease(percent * 2.0 - 1.0) * 0.5 + 0.5
                }
            }
            Easing::Step(steps) => {
                let steps = steps as f32;
                (percent * steps).floor() / steps
            }
            Easing::None => 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PropertyTween {
    data: PropertyTweenData,
    elapsed_seconds: f32,
}

#[derive(Clone, Debug)]
enum PropertyTweenData {
    Color {
        start: ColorF,
        end: ColorF,
        duration_seconds: f32,
        easing: Easing,
    },
    Coloring {
        start: Coloring,
        end: Coloring,
        duration_seconds: f32,
        easing: Easing,
    },
    Transform {
        start_scale: Vector2F,
        end_scale: Vector2F,
        start_translation: Vector2F,
        end_translation: Vector2F,
        start_theta: f32,
        end_theta: f32,
        duration_seconds: f32,
        easing: Easing,
    },
    ViewRect {
        start: RectPoints,
        end: RectPoints,
        duration_seconds: f32,
        easing: Easing,
    },
    MorphIndex {
        start: f32,
        end: f32,
        duration_seconds: f32,
        easing: Easing,
    },
}

impl PropertyTween {
    pub fn new_color(
        start: ColorU,
        end: ColorU,
        duration_seconds: f32,
        easing: Easing,
    ) -> PropertyTween {
        PropertyTween {
            data: PropertyTweenData::Color {
                start: start.to_f32(),
                end: end.to_f32(),
                duration_seconds,
                easing,
            },
            elapsed_seconds: 0.0,
        }
    }

    pub fn new_coloring(
        start: Coloring,
        end: Coloring,
        duration_seconds: f32,
        easing: Easing,
    ) -> PropertyTween {
        PropertyTween {
            data: PropertyTweenData::Coloring {
                start,
                end,
                duration_seconds,
                easing,
            },
            elapsed_seconds: 0.0,
        }
    }

    pub fn new_transform(
        start: ScaleRotationTranslation,
        end: ScaleRotationTranslation,
        duration_seconds: f32,
        easing: Easing,
    ) -> PropertyTween {
        PropertyTween {
            data: PropertyTweenData::Transform {
                start_scale: start.scale,
                end_scale: end.scale,
                start_translation: start.translation,
                end_translation: end.translation,
                start_theta: {
                    let delta = end.theta - start.theta;
                    if delta > PI {
                        start.theta + PI * 2.0
                    } else if delta < -PI {
                        start.theta - PI * 2.0
                    } else {
                        start.theta
                    }
                },
                end_theta: end.theta,
                duration_seconds,
                easing,
            },
            elapsed_seconds: 0.0,
        }
    }

    pub fn new_view_rect(
        start: RectPoints,
        end: RectPoints,
        duration_seconds: f32,
        easing: Easing,
    ) -> PropertyTween {
        PropertyTween {
            data: PropertyTweenData::ViewRect {
                start,
                end,
                duration_seconds,
                easing,
            },
            elapsed_seconds: 0.0,
        }
    }

    pub fn new_morph_index(
        start: f32,
        end: f32,
        duration_seconds: f32,
        easing: Easing,
    ) -> PropertyTween {
        PropertyTween {
            data: PropertyTweenData::MorphIndex {
                start: util::clamp_0_1(start),
                end: util::clamp_0_1(end),
                duration_seconds,
                easing,
            },
            elapsed_seconds: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum PropertyTweenUpdate {
    Color(ColorU),
    Coloring(Coloring),
    Transform(Transform2F),
    ViewRect(RectF),
    Morph(f32),
}

impl Tween for PropertyTween {
    type Item = PropertyTweenUpdate;

    fn update(&mut self, delta_time: f32) -> Self::Item {
        self.elapsed_seconds += delta_time;
        match &self.data {
            PropertyTweenData::Color {
                start,
                end,
                duration_seconds,
                easing,
            } => {
                let value = easing.ease(self.elapsed_seconds / duration_seconds);
                PropertyTweenUpdate::Color(start.lerp(*end, value).to_u8())
            }
            PropertyTweenData::Coloring {
                start,
                end,
                duration_seconds,
                easing,
            } => {
                let value = easing.ease(self.elapsed_seconds / duration_seconds);
                PropertyTweenUpdate::Coloring(start.lerp(end, value))
            }
            PropertyTweenData::Transform {
                start_scale,
                end_scale,
                start_translation,
                end_translation,
                start_theta,
                end_theta,
                duration_seconds,
                easing,
            } => {
                let value = easing.ease(self.elapsed_seconds / duration_seconds);
                PropertyTweenUpdate::Transform(Transform2F::from_scale_rotation_translation(
                    start_scale.lerp(*end_scale, value),
                    (end_theta - start_theta) * value + start_theta,
                    start_translation.lerp(*end_translation, value),
                ))
            }
            PropertyTweenData::ViewRect {
                start,
                end,
                duration_seconds,
                easing,
            } => {
                let value = easing.ease(self.elapsed_seconds / duration_seconds);
                PropertyTweenUpdate::ViewRect(RectF::from_points(
                    start.origin.lerp(end.origin, value),
                    start.lower_right.lerp(end.lower_right, value),
                ))
            }
            PropertyTweenData::MorphIndex {
                start,
                end,
                duration_seconds,
                easing,
            } => {
                let value = easing.ease(self.elapsed_seconds / duration_seconds);
                PropertyTweenUpdate::Morph(util::lerp(*start, *end, value))
            }
        }
    }
    fn is_complete(&self) -> bool {
        match self.data {
            PropertyTweenData::Color {
                duration_seconds, ..
            } => self.elapsed_seconds >= duration_seconds,
            PropertyTweenData::Coloring {
                duration_seconds, ..
            } => self.elapsed_seconds >= duration_seconds,
            PropertyTweenData::Transform {
                duration_seconds, ..
            } => self.elapsed_seconds >= duration_seconds,
            PropertyTweenData::ViewRect {
                duration_seconds, ..
            } => self.elapsed_seconds >= duration_seconds,
            PropertyTweenData::MorphIndex {
                duration_seconds, ..
            } => self.elapsed_seconds >= duration_seconds,
        }
    }
    fn easing(&self) -> Easing {
        match self.data {
            PropertyTweenData::Color { easing, .. } => easing,
            PropertyTweenData::Coloring { easing, .. } => easing,
            PropertyTweenData::Transform { easing, .. } => easing,
            PropertyTweenData::ViewRect { easing, .. } => easing,
            PropertyTweenData::MorphIndex { easing, .. } => easing,
        }
    }
}
