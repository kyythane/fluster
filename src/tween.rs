#![deny(clippy::all)]

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

impl Easing {
    pub fn ease(self, percent: f32) -> f32 {
        // clamp
        let percent = if percent > 1.0 {
            1.0
        } else if percent < 0.0 {
            0.0
        } else {
            percent
        };
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
                (percent * steps).floor() * steps
            }
            Easing::None => 1.0,
        }
    }
}
