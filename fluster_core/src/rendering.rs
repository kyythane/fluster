#![deny(clippy::all)]
use super::types::{transform_des, transform_ser, Vector2FDef};
use super::util;
use pathfinder_content::pattern::Pattern;
use pathfinder_color::ColorU;
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use serde::{Deserialize, Serialize};

pub trait Renderer {
    fn start_frame(&mut self, stage_size: Vector2F);
    fn set_background(&mut self, color: ColorU);
    fn draw_shape(
        &mut self,
        shape: &Shape,
        transform: Transform2F,
        color_override: &Option<Coloring>,
        morph_index: f32,
    );
    fn draw_raster(
        &mut self,
        pattern: &Pattern,
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
pub enum MorphPoint {
    Move(
        #[serde(with = "Vector2FDef")] Vector2F,
        #[serde(with = "Vector2FDef")] Vector2F,
    ),
    Line(
        #[serde(with = "Vector2FDef")] Vector2F,
        #[serde(with = "Vector2FDef")] Vector2F,
    ),
    Quadratic {
        #[serde(with = "Vector2FDef")]
        control_start: Vector2F,
        #[serde(with = "Vector2FDef")]
        to_start: Vector2F,
        #[serde(with = "Vector2FDef")]
        control_end: Vector2F,
        #[serde(with = "Vector2FDef")]
        to_end: Vector2F,
    },
    Bezier {
        #[serde(with = "Vector2FDef")]
        control_1_start: Vector2F,
        #[serde(with = "Vector2FDef")]
        control_2_start: Vector2F,
        #[serde(with = "Vector2FDef")]
        to_start: Vector2F,
        #[serde(with = "Vector2FDef")]
        control_1_end: Vector2F,
        #[serde(with = "Vector2FDef")]
        control_2_end: Vector2F,
        #[serde(with = "Vector2FDef")]
        to_end: Vector2F,
    },
    Arc {
        #[serde(with = "Vector2FDef")]
        control_start: Vector2F,
        #[serde(with = "Vector2FDef")]
        to_start: Vector2F,
        radius_start: f32,
        #[serde(with = "Vector2FDef")]
        control_end: Vector2F,
        #[serde(with = "Vector2FDef")]
        to_end: Vector2F,
        radius_end: f32,
    },
}

impl MorphPoint {
    pub fn to_point(&self, percent: f32) -> Point {
        match self {
            MorphPoint::Move(start, end) => Point::Move(start.lerp(*end, percent)),
            MorphPoint::Line(start, end) => Point::Line(start.lerp(*end, percent)),
            MorphPoint::Quadratic {
                control_start,
                to_start,
                control_end,
                to_end,
            } => Point::Quadratic {
                control: control_start.lerp(*control_end, percent),
                to: to_start.lerp(*to_end, percent),
            },
            MorphPoint::Bezier {
                control_1_start,
                control_2_start,
                to_start,
                control_1_end,
                control_2_end,
                to_end,
            } => Point::Bezier {
                control_1: control_1_start.lerp(*control_1_end, percent),
                control_2: control_2_start.lerp(*control_2_end, percent),
                to: to_start.lerp(*to_end, percent),
            },
            MorphPoint::Arc {
                control_start,
                to_start,
                radius_start,
                control_end,
                to_end,
                radius_end,
            } => Point::Arc {
                control: control_start.lerp(*control_end, percent),
                to: to_start.lerp(*to_end, percent),
                radius: util::lerp(*radius_start, *radius_end, percent),
            },
        }
    }
}

//TODO: since these vecs are immutable, replace with Box<[T]> (into_boxed_slice())
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
    Fill {
        points: Vec<Point>,
        #[serde(with = "ColorUDef")]
        color: ColorU,
    },
    MorphPath {
        points: Vec<MorphPoint>,
        #[serde(with = "ColorUDef")]
        color: ColorU,
        #[serde(with = "StrokeStyleDef")]
        stroke_style: StrokeStyle,
        is_closed: bool,
    },
    MorphFill {
        points: Vec<MorphPoint>,
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
            Shape::Fill { color, .. } => Coloring::Color(*color),
            Shape::MorphPath { color, .. } => Coloring::Color(*color),
            Shape::MorphFill { color, .. } => Coloring::Color(*color),
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
