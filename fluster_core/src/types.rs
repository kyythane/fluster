#![deny(clippy::all)]
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use pathfinder_simd::default::F32x2;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct ScaleRotationTranslation {
    #[serde(with = "Vector2FDef")]
    pub scale: Vector2F,
    pub theta: f32,
    #[serde(with = "Vector2FDef")]
    pub translation: Vector2F,
}

impl ScaleRotationTranslation {
    pub fn from_transform(transform: &Transform2F) -> ScaleRotationTranslation {
        let theta = transform.rotation();
        let cos_theta = theta.cos();
        ScaleRotationTranslation {
            scale: Vector2F::new(transform.m11() / cos_theta, transform.m22() / cos_theta),
            theta,
            translation: transform.translation(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Vector2F")]
pub struct Vector2FDef(#[serde(with = "F32x2Def")] pub F32x2);

#[derive(Serialize, Deserialize)]
#[serde(remote = "F32x2")]
pub struct F32x2Def(pub u64);

pub fn transform_des<'de, D>(deserializer: D) -> Result<Transform2F, D::Error>
where
    D: Deserializer<'de>,
{
    let srt = ScaleRotationTranslation::deserialize(deserializer)?;
    Ok(Transform2F::from_scale_rotation_translation(
        srt.scale,
        srt.theta,
        srt.translation,
    ))
}

pub fn transform_ser<S>(t: &Transform2F, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ScaleRotationTranslation::serialize(&ScaleRotationTranslation::from_transform(t), serializer)
}
