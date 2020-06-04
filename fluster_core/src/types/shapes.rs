use super::basic::{transform_des, transform_ser, ColorUDef, Vector2FDef};
use crate::util;
use pathfinder_canvas::Path2D;
use pathfinder_color::ColorU;
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::{rect::RectF, vector::Vector2F};
use reduce::Reduce;
use serde::{Deserialize, Serialize};

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
    Arc {
        #[serde(with = "Vector2FDef")]
        control: Vector2F,
        #[serde(with = "Vector2FDef")]
        to: Vector2F,
        radius: f32,
    },
}

impl Edge {
    pub fn end_point(&self) -> Vector2F {
        match self {
            Self::Move(v) => *v,
            Self::Line(v) => *v,
            Self::Quadratic { to, .. } | Self::Bezier { to, .. } | Self::Arc { to, .. } => *to,
        }
    }

    pub fn edges_to_path(edges: impl Iterator<Item = Edge>, is_closed: bool) -> Path2D {
        let mut path = Path2D::new();
        edges.for_each(|edge| match edge {
            Edge::Move(to) => path.move_to(to),
            Edge::Line(to) => path.line_to(to),
            Edge::Quadratic { control, to } => path.quadratic_curve_to(control, to),
            Edge::Bezier {
                control_1,
                control_2,
                to,
            } => path.bezier_curve_to(control_1, control_2, to),
            Edge::Arc {
                control,
                to,
                radius,
            } => path.arc_to(control, to, radius),
        });
        if is_closed {
            path.close_path();
        }
        path
    }

    fn compute_bounding(
        edges: impl Iterator<Item = Edge>,
        is_closed: bool,
        transform: &Transform2F,
    ) -> RectF {
        let mut outline = Self::edges_to_path(edges, is_closed).into_outline();
        outline.transform(transform);
        outline.bounds()
    }

    pub fn query_disk(
        &self,
        point: &Vector2F,
        radius: f32,
        transform: &Transform2F,
    ) -> impl Iterator<Item = (usize, Vector2F)> {
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
            Self::Arc { control, to, .. } => Self::match_points_disk(
                vec![*control, *to].into_iter(),
                *point,
                square_radius,
                *transform,
            ),
        }
    }

    fn match_points_disk(
        points: impl Iterator<Item = Vector2F>,
        point: Vector2F,
        square_radius: f32,
        transform: Transform2F,
    ) -> impl Iterator<Item = (usize, Vector2F)> {
        points
            .enumerate()
            .filter(move |p| (point - transform * p.1).square_length() <= square_radius)
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
            Self::Arc { control, to, .. } => {
                Self::match_points_rect(vec![*control, *to].into_iter(), *rect, *transform)
            }
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

impl MorphEdge {
    pub fn to_edge(&self, percent: f32) -> Edge {
        match self {
            MorphEdge::Move(start, end) => Edge::Move(start.lerp(*end, percent)),
            MorphEdge::Line(start, end) => Edge::Line(start.lerp(*end, percent)),
            MorphEdge::Quadratic {
                control_start,
                to_start,
                control_end,
                to_end,
            } => Edge::Quadratic {
                control: control_start.lerp(*control_end, percent),
                to: to_start.lerp(*to_end, percent),
            },
            MorphEdge::Bezier {
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
            MorphEdge::Arc {
                control_start,
                to_start,
                radius_start,
                control_end,
                to_end,
                radius_end,
            } => Edge::Arc {
                control: control_start.lerp(*control_end, percent),
                to: to_start.lerp(*to_end, percent),
                radius: util::lerp(*radius_start, *radius_end, percent),
            },
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Shape {
    Path {
        edges: Vec<Edge>,
        #[serde(with = "ColorUDef")]
        color: ColorU,
        #[serde(with = "StrokeStyleDef")]
        stroke_style: StrokeStyle,
        is_closed: bool,
    },
    Fill {
        edges: Vec<Edge>,
        #[serde(with = "ColorUDef")]
        color: ColorU,
    },
    MorphPath {
        edges: Vec<MorphEdge>,
        #[serde(with = "ColorUDef")]
        color: ColorU,
        #[serde(with = "StrokeStyleDef")]
        stroke_style: StrokeStyle,
        is_closed: bool,
    },
    MorphFill {
        edges: Vec<MorphEdge>,
        #[serde(with = "ColorUDef")]
        color: ColorU,
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
}

impl Shape {
    pub fn compute_bounding(&self, transform: &Transform2F, morph_percent: f32) -> RectF {
        match self {
            Shape::Path {
                edges, is_closed, ..
            } => Edge::compute_bounding(edges.iter().map(|e| *e), *is_closed, &transform),
            Shape::Fill { edges, .. } | Shape::Clip { edges, .. } => {
                Edge::compute_bounding(edges.iter().map(|e| *e), true, &transform)
            }
            Shape::MorphPath {
                edges: morph_edges,
                is_closed,
                ..
            } => {
                let edges = morph_edges
                    .iter()
                    .map(|morph_edge| morph_edge.to_edge(morph_percent));
                Edge::compute_bounding(edges, *is_closed, &transform)
            }
            Shape::MorphFill {
                edges: morph_edges, ..
            } => {
                let edges = morph_edges
                    .iter()
                    .map(|morph_edge| morph_edge.to_edge(morph_percent));
                Edge::compute_bounding(edges, true, &transform)
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
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Coloring {
    Color(#[serde(with = "ColorUDef")] ColorU),
    Colorings(Vec<Coloring>),
    //TODO: Gradient(Vector2F, Vec<ColorU>) ???
    //TODO: Patterns ???
    None,
}

impl Coloring {
    #[inline]
    //TODO: evaluate if tweens should operate using a proper linear space from palette?
    //If the Colorings don't match return None. In effect this means we'll return to the default Coloring of the shape.
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
