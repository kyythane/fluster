#![deny(clippy::all)]

#[inline]
pub fn clamp<T: PartialOrd>(n: T, min: T, max: T) -> T {
    if n > max {
        max
    } else if n < min {
        min
    } else {
        n
    }
}

#[inline]
pub fn clamp_0_1(n: f32) -> f32 {
    clamp(n, 0.0, 1.0)
}

#[inline]
pub fn lerp(s: f32, e: f32, p: f32) -> f32 {
    (e - s) * p + s
}
