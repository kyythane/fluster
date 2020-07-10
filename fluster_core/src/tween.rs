#![deny(clippy::all)]

use super::actions::RectPoints;
use super::types::{basic::ScaleRotationTranslation, coloring::Coloring};
use super::util;
use pathfinder_color::{ColorF, ColorU};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use serde::{Deserialize, Serialize};
use std::f32::{
    consts::{FRAC_PI_2, PI},
    EPSILON,
};
use std::mem;
use std::time::Duration;

pub trait Tween {
    type Item: ?Sized;

    fn compute(&self) -> Self::Item;
    fn update(&mut self, delta_frames: u32, delta_time: Duration);
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
                2f32.powf(percent * -10.0) * ((percent - 0.1) * 5.0 * PI).sin() + 1.0
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
                    2f32.powf((percent - 1.0) * -10.0) * ((percent - 1.1) * 5.0 * PI).sin() * 0.5
                        + 1.0
                }
            }
            Easing::BackIn => percent * percent * ((BACK_SCALE + 1.0) * percent - BACK_SCALE),
            Easing::BackOut => {
                let percent = percent - 1.0;
                percent * percent * ((BACK_SCALE + 1.0) * percent + BACK_SCALE) + 1.0
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
                    (percent * percent * ((IN_OUT_BACK_SCALE + 1.0) * percent + IN_OUT_BACK_SCALE)
                        + 2.0)
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
    elapsed: TweenElapsed,
    easing: Easing,
}

#[derive(Clone, Debug)]
pub enum TweenDuration {
    Time(Duration),
    Frame(u32),
}

#[derive(Clone, Debug)]
enum TweenElapsed {
    Time(Duration, Duration),
    Frame(u32, u32),
}

impl TweenElapsed {
    pub fn is_complete(&self) -> bool {
        match self {
            Self::Time(elapsed, max) => elapsed >= max,
            Self::Frame(elapsed, max) => elapsed >= max,
        }
    }

    pub fn as_percent(&self) -> f32 {
        match self {
            Self::Time(elapsed, max) => elapsed.div_duration_f32(*max),
            Self::Frame(elapsed, max) => *elapsed as f32 / *max as f32,
        }
    }
}

#[derive(Clone, Debug)]
pub enum PropertyTweenData {
    Color {
        start: ColorF,
        end: ColorF,
    },
    Coloring {
        start: Coloring,
        end: Coloring,
    },
    Transform {
        start_scale: Vector2F,
        end_scale: Vector2F,
        start_translation: Vector2F,
        end_translation: Vector2F,
        start_theta: f32,
        end_theta: f32,
    },
    ViewRect {
        start: RectPoints,
        end: RectPoints,
    },
    MorphIndex {
        start: f32,
        end: f32,
    },
}

// TODO: color and coloring twwen in sRGB. Investigate if using palette could help
impl PropertyTween {
    pub fn new_color(
        start: ColorU,
        end: ColorU,
        duration: TweenDuration,
        easing: Easing,
    ) -> PropertyTween {
        PropertyTween {
            data: PropertyTweenData::Color {
                start: start.to_f32(),
                end: end.to_f32(),
            },
            elapsed: Self::construct_elapsed(duration),
            easing,
        }
    }

    pub fn new_coloring(
        start: Coloring,
        end: Coloring,
        duration: TweenDuration,
        easing: Easing,
    ) -> PropertyTween {
        PropertyTween {
            data: PropertyTweenData::Coloring { start, end },
            elapsed: Self::construct_elapsed(duration),
            easing,
        }
    }

    pub fn new_transform(
        start: ScaleRotationTranslation,
        end: ScaleRotationTranslation,
        duration: TweenDuration,
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
            },
            elapsed: Self::construct_elapsed(duration),
            easing,
        }
    }

    pub fn new_view_rect(
        start: RectPoints,
        end: RectPoints,
        duration: TweenDuration,
        easing: Easing,
    ) -> PropertyTween {
        PropertyTween {
            data: PropertyTweenData::ViewRect { start, end },
            elapsed: Self::construct_elapsed(duration),
            easing,
        }
    }

    pub fn new_morph_index(
        start: f32,
        end: f32,
        duration: TweenDuration,
        easing: Easing,
    ) -> PropertyTween {
        PropertyTween {
            data: PropertyTweenData::MorphIndex {
                start: util::clamp_0_1(start),
                end: util::clamp_0_1(end),
            },
            elapsed: Self::construct_elapsed(duration),
            easing,
        }
    }

    fn construct_elapsed(duration: TweenDuration) -> TweenElapsed {
        match duration {
            TweenDuration::Time(max) => TweenElapsed::Time(Duration::from_millis(0), max),
            TweenDuration::Frame(max) => TweenElapsed::Frame(0, max),
        }
    }

    pub fn tween_data(&self) -> &PropertyTweenData {
        &self.data
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

    fn update(&mut self, delta_frames: u32, delta_time: Duration) {
        let elapsed = match &self.elapsed {
            TweenElapsed::Time(elapsed_time, max_time) => {
                let elapsed_time = if let Some(time) = elapsed_time.checked_add(delta_time) {
                    time
                } else {
                    *max_time
                };
                TweenElapsed::Time(elapsed_time, *max_time)
            }
            TweenElapsed::Frame(elapsed_frame, max_frame) => {
                TweenElapsed::Frame(elapsed_frame + delta_frames, *max_frame)
            }
        };
        mem::replace(&mut self.elapsed, elapsed);
    }

    fn compute(&self) -> Self::Item {
        match &self.data {
            PropertyTweenData::Color { start, end } => {
                let value = self.easing.ease(self.elapsed.as_percent());
                PropertyTweenUpdate::Color(start.lerp(*end, value).to_u8())
            }
            PropertyTweenData::Coloring { start, end } => {
                let value = self.easing.ease(self.elapsed.as_percent());
                PropertyTweenUpdate::Coloring(start.lerp(end, value))
            }
            PropertyTweenData::Transform {
                start_scale,
                end_scale,
                start_translation,
                end_translation,
                start_theta,
                end_theta,
            } => {
                let value = self.easing.ease(self.elapsed.as_percent());
                PropertyTweenUpdate::Transform(Transform2F::from_scale_rotation_translation(
                    start_scale.lerp(*end_scale, value),
                    (end_theta - start_theta) * value + start_theta,
                    start_translation.lerp(*end_translation, value),
                ))
            }
            PropertyTweenData::ViewRect { start, end } => {
                let value = self.easing.ease(self.elapsed.as_percent());
                PropertyTweenUpdate::ViewRect(RectF::from_points(
                    start.origin.lerp(end.origin, value),
                    start.lower_right.lerp(end.lower_right, value),
                ))
            }
            PropertyTweenData::MorphIndex { start, end } => {
                let value = self.easing.ease(self.elapsed.as_percent());
                PropertyTweenUpdate::Morph(util::lerp(*start, *end, value))
            }
        }
    }
    fn is_complete(&self) -> bool {
        self.elapsed.is_complete()
    }
    fn easing(&self) -> Easing {
        self.easing
    }
}
