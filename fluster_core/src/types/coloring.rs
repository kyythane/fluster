use palette::{Hsva, Laba, Lcha, LinSrgba, Mix};
use pathfinder_geometry::vector::Vector4F;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Div};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Coloring {
    Color(LinSrgba),
    Colorings(Vec<Coloring>),
    //TODO: Gradient
    None,
}

#[derive(Clone, Debug)]
pub enum DenormalizedColoring {
    Color(Vector4F),
    Colorings(Vec<DenormalizedColoring>),
    None,
}

impl DenormalizedColoring {
    pub fn into_coloring(&self) -> Coloring {
        match self {
            Self::Color(denormalized) => Coloring::Color(LinSrgba::from_components((
                denormalized.x(),
                denormalized.y(),
                denormalized.z(),
                denormalized.w(),
            ))),
            Self::Colorings(denormalized_colorings) => Coloring::Colorings(
                denormalized_colorings
                    .into_iter()
                    .map(|denormalized| denormalized.into_coloring())
                    .collect(),
            ),
            Self::None => Coloring::None,
        }
    }

    pub fn from_coloring(coloring: Coloring) -> DenormalizedColoring {
        coloring.into_denormalized()
    }
}

impl Add for DenormalizedColoring {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        match self {
            Self::Color(self_deno) => {
                if let Self::Color(other_deno) = other {
                    Self::Color(self_deno + other_deno)
                } else {
                    Self::None
                }
            }
            Self::Colorings(self_denos) => {
                if let Self::Colorings(other_denos) = other {
                    if self_denos.len() != other_denos.len() {
                        Self::None
                    } else {
                        let mut new_colorings: Vec<DenormalizedColoring> =
                            vec![DenormalizedColoring::None; self_denos.len()];
                        for i in 0..self_denos.len() {
                            new_colorings[i] = self_denos[i] + other_denos[i];
                        }
                        Self::Colorings(new_colorings)
                    }
                } else {
                    Self::None
                }
            }
            Self::None => Self::None,
        }
    }
}

impl Div<f32> for DenormalizedColoring {
    type Output = Self;

    fn div(self, rhs: f32) -> Self {
        match self {
            Self::Color(self_deno) => Self::Color(self_deno / rhs),
            Self::Colorings(self_denos) => Self::Colorings(
                self_denos
                    .into_iter()
                    .map(|denormalized| denormalized / rhs)
                    .collect(),
            ),
            Self::None => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ColorSpace {
    Linear,
    Hsv,
    Lab,
    Lch,
}

impl Coloring {
    pub fn into_denormalized(&self) -> DenormalizedColoring {
        match self {
            Self::Color(color) => {
                let components = color.into_components();
                DenormalizedColoring::Color(Vector4F::new(
                    components.0,
                    components.1,
                    components.2,
                    components.3,
                ))
            }
            Self::Colorings(colorings) => DenormalizedColoring::Colorings(
                colorings
                    .into_iter()
                    .map(|coloring| coloring.into_denormalized())
                    .collect(),
            ),
            Self::None => DenormalizedColoring::None,
        }
    }

    pub fn from_denormalized(denormalized: DenormalizedColoring) -> Coloring {
        denormalized.into_coloring()
    }

    // If the Colorings don't match return None. In effect this means we'll return to the default Coloring of the shape.
    pub fn lerp(&self, end: &Coloring, percent: f32, color_space: ColorSpace) -> Self {
        match self {
            Self::Color(start_color) => {
                if let Self::Color(end_color) = end {
                    let result_color = match color_space {
                        ColorSpace::Hsv => {
                            let start_color = Hsva::from(*start_color);
                            let end_color = Hsva::from(*end_color);
                            LinSrgba::from(start_color.mix(&end_color, percent))
                        }
                        ColorSpace::Lab => {
                            let start_color = Laba::from(*start_color);
                            let end_color = Laba::from(*end_color);
                            LinSrgba::from(start_color.mix(&end_color, percent))
                        }
                        ColorSpace::Lch => {
                            let start_color = Lcha::from(*start_color);
                            let end_color = Lcha::from(*end_color);
                            LinSrgba::from(start_color.mix(&end_color, percent))
                        }
                        ColorSpace::Linear => start_color.mix(end_color, percent),
                    };
                    Self::Color(result_color)
                } else {
                    Self::None
                }
            }
            Self::Colorings(start_colorings) => {
                if let Self::Colorings(end_colorings) = end {
                    if start_colorings.len() != end_colorings.len() {
                        Self::None
                    } else {
                        let mut new_colorings: Vec<Coloring> =
                            vec![Coloring::None; start_colorings.len()];
                        for i in 0..start_colorings.len() {
                            new_colorings[i] =
                                start_colorings[i].lerp(&end_colorings[i], percent, color_space);
                        }
                        Self::Colorings(new_colorings)
                    }
                } else {
                    Self::None
                }
            }
            Self::None => Self::None,
        }
    }
}
