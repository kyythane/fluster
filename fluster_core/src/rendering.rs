#![deny(clippy::all)]
use super::types::{transform_des, transform_ser, Vector2FDef};
use pathfinder_color::ColorU;
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use serde::{Deserialize, Serialize};
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

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Point {
    Move(#[serde(with = "Vector2FDef")] Vector2F),
    Line(#[serde(with = "Vector2FDef")] Vector2F),
    Quadratic {
        #[serde(with = "Vector2FDef")]
        control: Vector2F,
        #[serde(with = "Vector2FDef")]
        to: Vector2F,
    },
    Bezier {
        #[serde(with = "Vector2FDef")]
        control_1: Vector2F,
        #[serde(with = "Vector2FDef")]
        control_2: Vector2F,
        #[serde(with = "Vector2FDef")]
        to: Vector2F,
    },
    Arc {
        #[serde(with = "Vector2FDef")]
        control: Vector2F,
        #[serde(with = "Vector2FDef")]
        to: Vector2F,
        radius: f32,
    },
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Shape {
    Path {
        points: Vec<Point>,
        #[serde(with = "ColorUDef")]
        color: ColorU,
        #[serde(with = "StrokeStyleDef")]
        stroke_style: StrokeStyle,
        is_closed: bool,
    },
    FillPath {
        points: Vec<Point>,
        #[serde(with = "ColorUDef")]
        color: ColorU,
    },
    Clip {
        points: Vec<Point>,
    },
    Group {
        shapes: Vec<AugmentedShape>,
    },
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AugmentedShape {
    pub shape: Shape,
    #[serde(serialize_with = "transform_ser", deserialize_with = "transform_des")]
    pub transform: Transform2F,
}

impl Shape {
    pub fn color(&self) -> Coloring {
        match self {
            Shape::Path { color, .. } => Coloring::Color(*color),
            Shape::FillPath { color, .. } => Coloring::Color(*color),
            Shape::Clip { .. } => Coloring::None,
            Shape::Group { shapes } => {
                Coloring::Colorings(shapes.iter().map(|s| s.shape.color()).collect())
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

    pub fn lerp(&self, end: &Coloring, percent: f32) -> Coloring {
        match self {
            Coloring::Color(start_color) => {
                if let Coloring::Color(end_color) = end {
                    Coloring::Color(
                        start_color
                            .to_f32()
                            .lerp(end_color.to_f32(), percent)
                            .to_u8(),
                    )
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
