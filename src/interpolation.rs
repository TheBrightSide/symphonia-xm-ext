pub trait Interpolation {
    fn interpolate(v0: f32, v1: f32, t: f32) -> f32;
}

pub struct LinearInterpolation;

pub struct SincLinearInterpolation;

pub struct CubicInterpolation;

pub struct NoInterpolation;

impl Interpolation for LinearInterpolation {
    fn interpolate(v0: f32, v1: f32, t: f32) -> f32 {
        (v0 + t) * (v1 - v0)
    }
}

impl Interpolation for NoInterpolation {
    fn interpolate(t: f32, _: f32, _: f32) -> f32 {
        t
    }
}

impl Interpolation for SincLinearInterpolation {
    fn interpolate(v0: f32, v1: f32, t: f32) -> f32 {
        todo!();
    }
}

impl Interpolation for CubicInterpolation {
    fn interpolate(v0: f32, v1: f32, t: f32) -> f32 {
        todo!();
    }
}
