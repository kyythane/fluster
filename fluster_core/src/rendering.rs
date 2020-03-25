#![deny(clippy::all)]

use pathfinder_color::ColorU;
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use pathfinder_simd::default::F32x2;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::mem;

pub trait Renderer {
    fn start_frame(&mut self, stage_size: Vector2F);
    fn set_background(&mut self, color: ColorU);
    fn draw_shape(
        &mut self,
        shape: &Shape,
        transform: Transform2F,
        color_override: &Option<Coloring>,
    );
    fn draw_bitmap(
        &mut self,
        bitmap: &Bitmap,
        view_rect: RectF,
        transform: Transform2F,
        tint: Option<ColorU>,
    ); //TODO: filters?
    fn end_frame(&mut self);
}

/*TODO: move to a more complex shape definition system.
    Obviously this'll impact PropertyTweens and Parts as well
    Additional TODO that might be convolved with this: supporting curves in the Path
*/
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Shape {
    Path {
        #[serde(
            serialize_with = "vec_vector2f_ser",
            deserialize_with = "vec_vector2f_des"
        )]
        points: Vec<Vector2F>,
        #[serde(with = "ColorUDef")]
        color: ColorU,
        #[serde(with = "StrokeStyleDef")]
        stroke_style: StrokeStyle,
        is_closed: bool,
    },
    FillPath {
        #[serde(
            serialize_with = "vec_vector2f_ser",
            deserialize_with = "vec_vector2f_des"
        )]
        points: Vec<Vector2F>,
        #[serde(with = "ColorUDef")]
        color: ColorU,
    },
    Clip {
        #[serde(
            serialize_with = "vec_vector2f_ser",
            deserialize_with = "vec_vector2f_des"
        )]
        points: Vec<Vector2F>,
    },
    Group {
        shapes: Vec<Shape>,
    },
}

impl Shape {
    pub fn color(&self) -> Coloring {
        match self {
            Shape::Path { color, .. } => Coloring::Color(*color),
            Shape::FillPath { color, .. } => Coloring::Color(*color),
            Shape::Clip { .. } => Coloring::None,
            Shape::Group { shapes } => {
                Coloring::Colorings(shapes.iter().map(|s| s.color()).collect())
            }
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Coloring {
    Color(#[serde(with = "ColorUDef")] ColorU),
    Colorings(Vec<Coloring>),
    //TODO: Gradient!
    None,
}

impl Coloring {
    #[inline]
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn lerp_color(start: &ColorU, end: &ColorU, percent: f32) -> ColorU {
        ColorU::new(
            ((start.r - end.r) as f32 * percent).round() as u8 + start.r,
            ((start.g - end.g) as f32 * percent).round() as u8 + start.g,
            ((start.b - end.r) as f32 * percent).round() as u8 + start.b,
            ((start.a - end.a) as f32 * percent).round() as u8 + start.a,
        )
    }

    pub fn lerp(&self, end: &Coloring, percent: f32) -> Coloring {
        match self {
            Coloring::Color(start_color) => {
                if let Coloring::Color(end_color) = end {
                    Coloring::Color(Coloring::lerp_color(start_color, end_color, percent))
                } else {
                    Coloring::None
                }
            }
            Coloring::Colorings(start_colorings) => {
                if let Coloring::Colorings(end_colorings) = end {
                    if start_colorings.len() != end_colorings.len() {
                        Coloring::None
                    } else {
                        let mut new_colorings: Vec<Coloring> =
                            vec![Coloring::None; start_colorings.len()];
                        for i in 0..start_colorings.len() {
                            new_colorings[i] = start_colorings[i].lerp(&end_colorings[i], percent);
                        }
                        Coloring::Colorings(new_colorings)
                    }
                } else {
                    Coloring::None
                }
            }
            Coloring::None => Coloring::None,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Bitmap {
    #[serde(with = "Vector2FDef")]
    dimensions: Vector2F,
    bytes: Vec<u8>,
}

impl Bitmap {
    pub fn release_contents(&mut self) -> Bitmap {
        Bitmap {
            dimensions: self.dimensions,
            bytes: mem::replace(&mut self.bytes, vec![]),
        }
    }
}

//The following deffinitions add serde support to pathfinder types
#[derive(Serialize, Deserialize)]
struct VecWrapper(#[serde(with = "Vector2FDef")] Vector2F);

fn vec_vector2f_des<'de, D>(deserializer: D) -> Result<Vec<Vector2F>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Vec::deserialize(deserializer)?;
    Ok(v.into_iter().map(|VecWrapper(a)| a).collect())
}

fn vec_vector2f_ser<S>(v: &[Vector2F], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(v.len()))?;
    for element in v.iter().map(|a| VecWrapper(*a)) {
        seq.serialize_element(&element)?;
    }
    seq.end()
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Vector2F")]
pub struct Vector2FDef(#[serde(with = "F32x2Def")] pub F32x2);

#[derive(Serialize, Deserialize)]
#[serde(remote = "F32x2")]
pub struct F32x2Def(pub u64);

#[derive(Serialize, Deserialize)]
#[serde(remote = "ColorU")]
pub struct ColorUDef {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "StrokeStyle")]
pub struct StrokeStyleDef {
    pub line_width: f32,
    #[serde(with = "LineCapDef")]
    pub line_cap: LineCap,
    #[serde(with = "LineJoinDef")]
    pub line_join: LineJoin,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "LineCap")]
pub enum LineCapDef {
    Butt,
    Square,
    Round,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "LineJoin")]
pub enum LineJoinDef {
    Miter(f32),
    Bevel,
    Round,
}
