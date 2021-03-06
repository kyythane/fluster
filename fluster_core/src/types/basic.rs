use pathfinder_color::ColorU;
use pathfinder_content::pattern::{Image, Pattern};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use pathfinder_simd::default::F32x2;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_bytes::{ByteBuf, Bytes};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct ScaleRotationTranslation {
    #[serde(with = "Vector2FDef")]
    pub scale: Vector2F,
    pub theta: f32,
    #[serde(with = "Vector2FDef")]
    pub translation: Vector2F,
}

impl ScaleRotationTranslation {
    pub fn new(scale: Vector2F, theta: f32, translation: Vector2F) -> Self {
        Self {
            scale,
            theta,
            translation,
        }
    }

    pub fn from_transform(transform: Transform2F) -> ScaleRotationTranslation {
        let theta = transform.rotation();
        let cos_theta = theta.cos();
        ScaleRotationTranslation {
            scale: Vector2F::new(transform.m11() / cos_theta, transform.m22() / cos_theta),
            theta,
            translation: transform.translation(),
        }
    }
}

impl Default for ScaleRotationTranslation {
    fn default() -> Self {
        ScaleRotationTranslation {
            scale: Vector2F::splat(1.0),
            theta: 0.0,
            translation: Vector2F::zero(),
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
    ScaleRotationTranslation::serialize(&ScaleRotationTranslation::from_transform(*t), serializer)
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Bitmap {
    pub size_x: i32,
    pub size_y: i32,
    #[serde(
        serialize_with = "bitmap_contents_ser",
        deserialize_with = "bitmap_contents_des"
    )]
    pub colors: Arc<Vec<ColorU>>,
}

impl Bitmap {
    pub fn pattern(&mut self) -> Pattern {
        let image = Image::new(Vector2I::new(self.size_x, self.size_y), self.colors.clone());
        Pattern::from_image(image)
    }
}

pub fn bitmap_contents_des<'de, D>(deserializer: D) -> Result<Arc<Vec<ColorU>>, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes = ByteBuf::deserialize(deserializer)?;
    let colors = pathfinder_color::u8_vec_to_color_vec(bytes.into_vec());
    Ok(Arc::new(colors))
}

pub fn bitmap_contents_ser<S>(a_c: &Arc<Vec<ColorU>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let bytes = pathfinder_color::color_slice_to_u8_slice(&a_c);
    Bytes::serialize(&Bytes::new(&bytes), serializer)
}

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ContainerId(Uuid);

impl ContainerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Into<Uuid> for ContainerId {
    fn into(self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for ContainerId {
    fn from(uuid: Uuid) -> Self {
        Self::from_uuid(uuid)
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct LibraryId(Uuid);

impl LibraryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Into<Uuid> for LibraryId {
    fn into(self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for LibraryId {
    fn from(uuid: Uuid) -> Self {
        Self::from_uuid(uuid)
    }
}
