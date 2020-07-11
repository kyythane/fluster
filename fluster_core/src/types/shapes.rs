use super::{
    basic::{transform_des, transform_ser, Vector2FDef},
    coloring::Coloring,
};
use crate::util;
use palette::LinSrgba;
use pathfinder_canvas::Path2D;
use pathfinder_content::{
    outline::ArcDirection,
    stroke::{LineCap, LineJoin, StrokeStyle},
};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::{rect::RectF, vector::Vector2F};
use reduce::Reduce;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::mem;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Edge {
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
    ArcTo {
        #[serde(with = "Vector2FDef")]
        control: Vector2F,
        #[serde(with = "Vector2FDef")]
        to: Vector2F,
        radius: f32,
    },
    Arc {
        #[serde(with = "Vector2FDef")]
        center: Vector2F,
        start_angle: f32,
        end_angle: f32,
        #[serde(with = "Vector2FDef")]
        axes: Vector2F,
        // TOOD: ArcDirection
    },
    Close,
}

impl Edge {
    pub fn new_ellipse(axes: Vector2F, transform: Transform2F) -> Vec<Self> {
        vec![
            Self::Move(transform * Vector2F::new(axes.x(), 0.0)),
            Self::Arc {
                center: transform * Vector2F::zero(),
                axes,
                start_angle: 0.0,
                end_angle: PI * 2.0,
            },
            Self::Close,
        ]
    }

    pub fn new_polygon(sides: u8, edge_length: f32, transform: Transform2F) -> Vec<Self> {
        let mut edges = Vec::with_capacity(sides as usize);
        let angle = Transform2F::from_rotation(PI - (sides as f32 - 2.0) * PI / sides as f32);
        let mut edge = Vector2F::new(edge_length, 0.0);
        let mut curr = Vector2F::zero();
        edges.push(Self::Move(transform * Vector2F::zero()));
        for _ in 0..sides {
            curr = curr + edge;
            edges.push(Self::Line(transform * curr));
            edge = angle * edge;
        }
        edges.push(Self::Close);
        edges
    }

    pub fn new_round_polygon(
        sides: u8,
        edge_length: f32,
        corner_radius: f32,
        transform: Transform2F,
    ) -> Vec<Self> {
        // edge_length and corner_radius must always be non-zero or we'll get NaNs
        let edge_length = edge_length.max(0.001);
        let f_sides = sides as f32;
        // corner_radius needs to be capped as a fraction of edge_length otherwise things look Weird
        let corner_radius = if sides == 3 {
            // Triangles are a special case because their corner_offset is larger than the radius!
            corner_radius.min(edge_length / 4.0)
        } else {
            // All other polygons are happy with capping radius at half
            corner_radius.min(edge_length / 2.0)
        };
        let corner_chord =
            (2.0 * corner_radius * corner_radius * (1.0 - (2.0 * PI / f_sides).cos())).sqrt();
        let corner_offset = (PI / f_sides).sin() / (2.0 * PI / f_sides).sin() * corner_chord;
        let mut edges = Vec::with_capacity(sides as usize);
        let angle = Transform2F::from_rotation(PI - (f_sides - 2.0) * PI / f_sides);
        let mut edge = Vector2F::new(edge_length, 0.0);
        let mut curr = Vector2F::zero();
        edges.push(Self::Move(transform * Vector2F::new(corner_offset, 0.0)));
        for _ in 0..sides {
            curr = curr + edge;
            edges.push(Self::ArcTo {
                control: transform * curr,
                to: transform * (curr + (angle * edge).normalize() * corner_offset),
                radius: corner_radius,
            });
            edge = angle * edge;
        }
        edges.push(Self::Close);
        edges
    }

    pub fn new_rect(size: Vector2F, transform: Transform2F) -> Vec<Self> {
        vec![
            Self::Move(transform * Vector2F::zero()),
            Self::Line(transform * Vector2F::new(size.x(), 0.0)),
            Self::Line(transform * size),
            Self::Line(transform * Vector2F::new(0.0, size.y())),
            Self::Line(transform * Vector2F::zero()), //Add the last line since we don't know if this will be closed
            Self::Close,
        ]
    }

    pub fn new_round_rect(size: Vector2F, corner_radius: f32, transform: Transform2F) -> Vec<Self> {
        let corner_radius = corner_radius
            .min(size.x() / 2.0)
            .min(size.y() / 2.0)
            .max(0.001); // We need to provides some minimum size so Pathfinder doesn't generate points with NaN coordinates
        vec![
            Self::Move(transform * (Vector2F::zero() + Vector2F::new(corner_radius, 0.0))),
            Self::ArcTo {
                control: transform * Vector2F::new(size.x(), 0.0),
                to: transform * Vector2F::new(size.x(), corner_radius),
                radius: corner_radius,
            },
            Self::ArcTo {
                control: transform * size,
                to: transform * (size + Vector2F::new(-corner_radius, 0.0)),
                radius: corner_radius,
            },
            Self::ArcTo {
                control: transform * Vector2F::new(0.0, size.y()),
                to: transform * Vector2F::new(0.0, size.y() - corner_radius),
                radius: corner_radius,
            },
            Self::ArcTo {
                control: transform * Vector2F::zero(),
                to: transform * Vector2F::new(corner_radius, 0.0),
                radius: corner_radius,
            },
            Self::Close,
        ]
    }

    pub fn new_superellipse(size: Vector2F, exponent: f32, transform: Transform2F) -> Vec<Self> {
        let size = size / 2.0;
        let transform = Transform2F::from_translation(size) * transform;
        const MAX_STEP: usize = 120;
        const STEP_SIZE: f32 = 2.0 * PI / MAX_STEP as f32;
        let key_points = (0..MAX_STEP)
            .map(move |step| {
                let arc_step = (step as f32) * STEP_SIZE;
                let x =
                    size.x() * arc_step.cos().signum() * arc_step.cos().abs().powf(2.0 / exponent);
                let y =
                    size.y() * arc_step.sin().signum() * arc_step.sin().abs().powf(2.0 / exponent);
                Vector2F::new(x, y)
            })
            .collect::<Vec<Vector2F>>();
        let mut edges = vec![Self::Move(transform * key_points[0])];
        fn compute_control_point(p_0: Vector2F, p_mid: Vector2F, p_1: Vector2F) -> Vector2F {
            p_mid * 2.0 - p_0 * 0.5 - p_1 * 0.5
        }
        for index in (2..MAX_STEP).step_by(3) {
            edges.push(Self::Quadratic {
                control: transform
                    * compute_control_point(
                        key_points[index - 2],
                        key_points[index - 1],
                        key_points[index],
                    ),
                to: transform * key_points[index],
            });
        }
        edges.push(Self::Quadratic {
            control: transform
                * compute_control_point(
                    key_points[MAX_STEP - 2],
                    key_points[MAX_STEP - 1],
                    key_points[0],
                ),
            to: transform * key_points[0],
        });
        edges.push(Self::Close);
        edges
    }

    pub fn end_point(&self) -> Vector2F {
        match self {
            Self::Move(v) => *v,
            Self::Line(v) => *v,
            Self::Quadratic { to, .. } | Self::Bezier { to, .. } | Self::ArcTo { to, .. } => *to,
            Self::Arc { .. } => Vector2F::zero(), // TODO: compute
            Self::Close { .. } => Vector2F::zero(), //TODO: Uh.... What should this endpoint be
        }
    }

    pub fn edges_to_path(edges: impl Iterator<Item = Edge>) -> Path2D {
        let mut path = Path2D::new();
        edges.for_each(|edge| match edge {
            Self::Move(to) => path.move_to(to),
            Self::Line(to) => path.line_to(to),
            Self::Quadratic { control, to } => path.quadratic_curve_to(control, to),
            Self::Bezier {
                control_1,
                control_2,
                to,
            } => path.bezier_curve_to(control_1, control_2, to),
            Self::ArcTo {
                control,
                to,
                radius,
            } => path.arc_to(control, to, radius),
            Self::Arc {
                center,
                start_angle,
                end_angle,
                axes,
            } => {
                if (axes.x() - axes.y()).abs() <= std::f32::EPSILON {
                    path.arc(center, axes.x(), start_angle, end_angle, ArcDirection::CW);
                } else {
                    path.ellipse(center, axes, 0.0, start_angle, end_angle);
                }
            }
            Self::Close => path.close_path(),
        });
        path
    }

    pub fn update_point(&mut self, index: usize, position: Vector2F) {
        let updated = match self {
            Self::Move(..) => Self::Move(position),
            Self::Line(..) => Self::Line(position),
            Self::Quadratic { control, to } => {
                if index == 0 {
                    Self::Quadratic {
                        control: position,
                        to: *to,
                    }
                } else {
                    Self::Quadratic {
                        control: *control,
                        to: position,
                    }
                }
            }
            Self::Bezier {
                control_1,
                control_2,
                to,
            } => match index {
                0 => Self::Bezier {
                    control_1: position,
                    control_2: *control_2,
                    to: *to,
                },
                1 => Self::Bezier {
                    control_1: *control_1,
                    control_2: position,
                    to: *to,
                },
                _ => Self::Bezier {
                    control_1: *control_1,
                    control_2: *control_2,
                    to: position,
                },
            },
            Self::ArcTo {
                control,
                to,
                radius,
            } => {
                if index == 0 {
                    Self::ArcTo {
                        control: position,
                        to: *to,
                        radius: *radius,
                    }
                } else {
                    Self::ArcTo {
                        control: *control,
                        to: position,
                        radius: *radius,
                    }
                }
            }
            Self::Arc { .. } => todo!(),
            Self::Close => Self::Close,
        };
        *self = updated;
    }

    fn compute_bounding(edges: impl Iterator<Item = Edge>, transform: &Transform2F) -> RectF {
        let mut outline = Self::edges_to_path(edges).into_outline();
        outline.transform(transform);
        outline.bounds()
    }

    pub fn query_disk(
        &self,
        point: &Vector2F,
        radius: f32,
        transform: &Transform2F,
    ) -> impl Iterator<Item = (usize, f32, Vector2F)> {
        let square_radius = radius * radius;
        match self {
            Self::Move(to) | Self::Line(to) => {
                Self::match_points_disk(vec![*to].into_iter(), *point, square_radius, *transform)
            }
            Self::Quadratic { control, to } => Self::match_points_disk(
                vec![*control, *to].into_iter(),
                *point,
                square_radius,
                *transform,
            ),
            Self::Bezier {
                control_1,
                control_2,
                to,
            } => Self::match_points_disk(
                vec![*control_1, *control_2, *to].into_iter(),
                *point,
                square_radius,
                *transform,
            ),
            // TODO: oh no radius!!!!???
            Self::ArcTo { control, to, .. } => Self::match_points_disk(
                vec![*control, *to].into_iter(),
                *point,
                square_radius,
                *transform,
            ),
            Self::Arc { center, .. } => Self::match_points_disk(
                vec![*center].into_iter(),
                *point,
                square_radius,
                *transform,
            ), // TODO, calc start and end of arc
            Self::Close => {
                Self::match_points_disk(vec![].into_iter(), *point, square_radius, *transform)
            }
        }
    }

    fn match_points_disk(
        points: impl Iterator<Item = Vector2F>,
        point: Vector2F,
        square_radius: f32,
        transform: Transform2F,
    ) -> impl Iterator<Item = (usize, f32, Vector2F)> {
        points
            .enumerate()
            .map(move |(index, p)| (index, (point - transform * p).square_length(), p))
            .filter(move |p| p.1 <= square_radius)
    }

    fn match_points_rect(
        points: impl Iterator<Item = Vector2F>,
        rect: RectF,
        transform: Transform2F,
    ) -> impl Iterator<Item = (usize, Vector2F)> {
        points
            .enumerate()
            .filter(move |p| rect.contains_point(transform * p.1))
    }

    pub fn query_rect(
        &self,
        rect: &RectF,
        transform: &Transform2F,
    ) -> impl Iterator<Item = (usize, Vector2F)> {
        match self {
            Self::Move(to) | Self::Line(to) => {
                Self::match_points_rect(vec![*to].into_iter(), *rect, *transform)
            }
            Self::Quadratic { control, to } => {
                Self::match_points_rect(vec![*control, *to].into_iter(), *rect, *transform)
            }
            Self::Bezier {
                control_1,
                control_2,
                to,
            } => Self::match_points_rect(
                vec![*control_1, *control_2, *to].into_iter(),
                *rect,
                *transform,
            ),
            // TODO: oh no radius!!!!???
            Self::ArcTo { control, to, .. } => {
                Self::match_points_rect(vec![*control, *to].into_iter(), *rect, *transform)
            }
            Self::Arc { center, .. } => {
                Self::match_points_rect(vec![*center].into_iter(), *rect, *transform)
            } // TODO, start end
            Self::Close => Self::match_points_rect(vec![].into_iter(), *rect, *transform),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum MorphEdge {
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
    ArcTo {
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
    Close,
}

impl MorphEdge {
    pub fn to_edge(&self, percent: f32) -> Edge {
        match self {
            Self::Move(start, end) => Edge::Move(start.lerp(*end, percent)),
            Self::Line(start, end) => Edge::Line(start.lerp(*end, percent)),
            Self::Quadratic {
                control_start,
                to_start,
                control_end,
                to_end,
            } => Edge::Quadratic {
                control: control_start.lerp(*control_end, percent),
                to: to_start.lerp(*to_end, percent),
            },
            Self::Bezier {
                control_1_start,
                control_2_start,
                to_start,
                control_1_end,
                control_2_end,
                to_end,
            } => Edge::Bezier {
                control_1: control_1_start.lerp(*control_1_end, percent),
                control_2: control_2_start.lerp(*control_2_end, percent),
                to: to_start.lerp(*to_end, percent),
            },
            Self::ArcTo {
                control_start,
                to_start,
                radius_start,
                control_end,
                to_end,
                radius_end,
            } => Edge::ArcTo {
                control: control_start.lerp(*control_end, percent),
                to: to_start.lerp(*to_end, percent),
                radius: util::lerp(*radius_start, *radius_end, percent),
            },
            Self::Close => Edge::Close,
        }
    }
}

//TODO: should close path be an instruction in edge???
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Shape {
    Path {
        edges: Vec<Edge>,
        color: LinSrgba,
        #[serde(with = "StrokeStyleDef")]
        stroke_style: StrokeStyle,
    },
    Fill {
        edges: Vec<Edge>,
        color: LinSrgba,
    },
    MorphPath {
        edges: Vec<MorphEdge>,
        color: LinSrgba,
        #[serde(with = "StrokeStyleDef")]
        stroke_style: StrokeStyle,
    },
    MorphFill {
        edges: Vec<MorphEdge>,
        color: LinSrgba,
    },
    Clip {
        edges: Vec<Edge>,
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

impl AugmentedShape {
    pub fn compute_bounding(&self, transform: &Transform2F, morph_percent: f32) -> RectF {
        self.shape
            .compute_bounding(&(*transform * self.transform), morph_percent)
    }

    pub fn edge_list(&self, morph_percent: f32) -> Vec<Edge> {
        self.shape.edge_list(morph_percent)
    }

    pub fn len(&self) -> usize {
        self.shape.len()
    }
}

impl Shape {
    pub fn compute_bounding(&self, transform: &Transform2F, morph_percent: f32) -> RectF {
        match self {
            Shape::Path { edges, .. } | Shape::Fill { edges, .. } | Shape::Clip { edges, .. } => {
                Edge::compute_bounding(edges.iter().map(|e| *e), &transform)
            }
            Shape::MorphPath {
                edges: morph_edges, ..
            }
            | Shape::MorphFill {
                edges: morph_edges, ..
            } => {
                let edges = morph_edges
                    .iter()
                    .map(|morph_edge| morph_edge.to_edge(morph_percent));
                Edge::compute_bounding(edges, &transform)
            }
            Shape::Group { shapes } => shapes
                .iter()
                .map(|s| s.compute_bounding(transform, morph_percent))
                .reduce(|a, b| a.union_rect(b))
                .unwrap(),
        }
    }

    pub fn edge_list(&self, morph_percent: f32) -> Vec<Edge> {
        match self {
            Shape::Path { edges, .. } | Shape::Fill { edges, .. } | Shape::Clip { edges, .. } => {
                edges.to_vec()
            }
            Shape::MorphPath {
                edges: morph_edges, ..
            }
            | Shape::MorphFill {
                edges: morph_edges, ..
            } => morph_edges
                .iter()
                .map(|morph_edge| morph_edge.to_edge(morph_percent))
                .collect::<Vec<Edge>>(),
            Shape::Group { shapes } => shapes
                .iter()
                .flat_map(|s| s.edge_list(morph_percent))
                .collect::<Vec<Edge>>(),
        }
    }

    pub fn color(&self) -> Coloring {
        match self {
            Shape::Path { color, .. }
            | Shape::Fill { color, .. }
            | Shape::MorphPath { color, .. }
            | Shape::MorphFill { color, .. } => Coloring::Color(*color),
            Shape::Clip { .. } => Coloring::None,
            Shape::Group { shapes } => {
                Coloring::Colorings(shapes.iter().map(|s| s.shape.color()).collect())
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Shape::Path { edges, .. } | Shape::Fill { edges, .. } | Shape::Clip { edges, .. } => {
                edges.len()
            }
            Shape::MorphPath {
                edges: morph_edges, ..
            }
            | Shape::MorphFill {
                edges: morph_edges, ..
            } => morph_edges.len(),
            Shape::Group { shapes } => shapes
                .iter()
                .map(|s| s.len())
                .reduce(|l, acc| l + acc)
                .unwrap_or_else(|| 0),
        }
    }
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
