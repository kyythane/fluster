use crate::actions::{EntityUpdateDefinition, PartUpdateDefinition};
use crate::tween::PropertyTween;
use pathfinder_color::ColorU;
use pathfinder_geometry::{transform2d::Transform2F, vector::Vector2F};
use reduce::Reduce;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Part {
    item_id: Uuid,
    transform: Transform2F,
    bounding_box: RectF,
    dirty: bool,
    active: bool,
    tweens: Vec<PropertyTween>,
    meta_data: PartMetaData,
    collision_layers: HashSet<QuadTreeLayer>,
}

//TODO: revist parts. Convert to struct (item_id, transfom, bounding_box, dirty, active) w/ metadata enum
#[derive(Clone, PartialEq, Debug)]
pub enum PartMetaData {
    Vector {
        color: Option<Coloring>,
    },
    Raster {
        view_rect: RectF,
        tint: Option<ColorU>,
    },
}

impl Part {
    pub fn recompute_bounds(
        &mut self,
        world_transform: &Transform2F,
        morph_percent: f32,
        library: &HashMap<Uuid, DisplayLibraryItem>,
    ) -> RectF {
        self.bounding_box = match self.meta_data {
            PartMetaData::Vector { .. } => library
                .get(&self.item_id)
                .unwrap()
                .compute_bounding(&(*world_transform * self.transform), morph_percent),
            PartMetaData::Raster { view_rect, .. } => {
                let transform = *world_transform * self.transform;
                let o = transform * Vector2F::default();
                let lr = transform * view_rect.size();
                RectF::from_points(o.min(lr), o.max(lr))
            }
        };
        self.mark_clean();
        self.bounding_box
    }

    pub fn add_tween(
        &mut self,
        library_item: &DisplayLibraryItem,
        part_update: &PartUpdateDefinition,
        duration_seconds: f32,
    ) -> Result<(), String> {
        /*let tween = match part_update.payload() {
            PartUpdatePayload::Transform(end_transform) => PropertyTween::new_transform(
                ScaleRotationTranslation::from_transform(&self.transform),
                *end_transform,
                duration_seconds,
                part_update.easing(),
            ),
            PartUpdatePayload::Coloring(end_color) => {
                if let PartMetaData::Vector { color } = &self.meta_data {
                    if let Some(start_color) = color {
                        PropertyTween::new_coloring(
                            start_color.clone(),
                            end_color.clone(),
                            duration_seconds,
                            part_update.easing(),
                        )
                    } else if let DisplayLibraryItem::Vector(shape) = library_item {
                        PropertyTween::new_coloring(
                            shape.color(),
                            end_color.clone(),
                            duration_seconds,
                            part_update.easing(),
                        )
                    } else {
                        return Err(format!(
                            "Vector part {} references a Bitmap object",
                            self.item_id()
                        ));
                    }
                } else {
                    return Err("Tried to apply Vector update to a Bitmap part".to_owned());
                }
            }
            PartUpdatePayload::ViewRect(end_view_rect) => {
                if let PartMetaData::Raster { view_rect, .. } = &self.meta_data {
                    PropertyTween::new_view_rect(
                        RectPoints::from_rect(view_rect),
                        *end_view_rect,
                        duration_seconds,
                        part_update.easing(),
                    )
                } else {
                    return Err("Tried to apply Bitmap update to a Vector part".to_owned());
                }
            }
            PartUpdatePayload::Tint(end_tint) => {
                if let PartMetaData::Raster { tint, .. } = self.meta_data {
                    if let Some(start_tint) = tint {
                        PropertyTween::new_color(
                            start_tint,
                            *end_tint,
                            duration_seconds,
                            part_update.easing(),
                        )
                    } else {
                        PropertyTween::new_color(
                            ColorU::white(),
                            *end_tint,
                            duration_seconds,
                            part_update.easing(),
                        )
                    }
                } else {
                    return Err("Tried to apply Bitmap update to a Vector part".to_owned());
                }
            }
        };
        self.tweens.push(tween);*/
        Ok(())
    }
}
