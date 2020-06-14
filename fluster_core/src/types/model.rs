use super::basic::ScaleRotationTranslation;
use super::shapes::{Coloring, Shape};
use crate::actions::{
    EntityUpdateDefinition, EntityUpdatePayload, PartUpdateDefinition, PartUpdatePayload,
    RectPoints,
};
use crate::tween::{PropertyTween, PropertyTweenUpdate, Tween};
use pathfinder_color::ColorU;
use pathfinder_content::pattern::Pattern;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::{transform2d::Transform2F, vector::Vector2F};
use reduce::Reduce;
use std::collections::HashMap;
use uuid::Uuid;
#[derive(Clone, PartialEq, Debug)]
pub enum DisplayLibraryItem {
    Vector(Shape),
    Raster(Pattern),
}

impl DisplayLibraryItem {
    pub fn compute_bounding(&self, transform: &Transform2F, morph_percent: f32) -> RectF {
        match self {
            Self::Vector(shape) => shape.compute_bounding(transform, morph_percent),
            Self::Raster(pattern) => {
                let transform = *transform;
                let o = transform * Vector2F::default();
                let lr = transform * pattern.size().to_f32();
                RectF::from_points(o.min(lr), o.max(lr))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Part {
    item_id: Uuid,
    transform: Transform2F,
    bounding_box: RectF,
    dirty: bool,
    active: bool,
    tweens: Vec<PropertyTween>,
    meta_data: PartMetaData,
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
    pub fn new_vector(item_id: Uuid, transform: Transform2F, color: Option<Coloring>) -> Self {
        Self {
            item_id,
            transform,
            bounding_box: RectF::default(),
            dirty: true,
            active: true,
            tweens: vec![],
            meta_data: PartMetaData::Vector { color },
        }
    }

    pub fn new_raster(
        item_id: Uuid,
        view_rect: RectF,
        transform: Transform2F,
        tint: Option<ColorU>,
    ) -> Part {
        Self {
            item_id,
            transform,
            bounding_box: RectF::default(),
            dirty: true,
            active: true,
            tweens: vec![],
            meta_data: PartMetaData::Raster { view_rect, tint },
        }
    }

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

    pub fn bounds(&self) -> &RectF {
        &self.bounding_box
    }

    pub fn item_id(&self) -> &Uuid {
        &self.item_id
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn activate(&mut self) {
        self.active = true;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn meta_data(&self) -> &PartMetaData {
        &self.meta_data
    }

    pub fn transform(&self) -> &Transform2F {
        &self.transform
    }

    pub fn add_tween(
        &mut self,
        library_item: &DisplayLibraryItem,
        part_update: &PartUpdateDefinition,
        duration_seconds: f32,
    ) -> Result<(), String> {
        let tween = match part_update.payload() {
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
        self.tweens.push(tween);
        Ok(())
    }

    pub fn update_tweens(&mut self, elapsed: f32) -> Result<bool, String> {
        let mut update_bounds = false;
        if let Some((new_transform, new_color, new_coloring, new_view_rect)) = self
            .tweens
            .iter_mut()
            .map(|tween| {
                let update = tween.update(elapsed);
                match update {
                    PropertyTweenUpdate::Transform(transform) => {
                        (Some(transform), None, None, None)
                    }
                    PropertyTweenUpdate::Color(color) => (None, Some(color), None, None),
                    PropertyTweenUpdate::Coloring(coloring) => (None, None, Some(coloring), None),
                    PropertyTweenUpdate::ViewRect(view_rect) => (None, None, None, Some(view_rect)),
                    // Morph updates are not valid for parts
                    PropertyTweenUpdate::Morph(_) => (None, None, None, None),
                }
            })
            .reduce(|acc, x| {
                let t = if let Some(transform) = x.0 {
                    if let Some(transform_acc) = acc.0 {
                        Some(transform * transform_acc)
                    } else {
                        x.0
                    }
                } else {
                    acc.0
                };
                let c = if x.1.is_some() { x.1 } else { acc.1 };
                let cs = if x.2.is_some() { x.2 } else { acc.2 };
                let v = if x.3.is_some() { x.3 } else { acc.3 };
                (t, c, cs, v)
            })
        {
            if let Some(transform) = new_transform {
                self.transform = transform;
                self.mark_dirty();
                update_bounds = true;
            };
            if let Some(tint) = new_color {
                self.meta_data = if let PartMetaData::Raster { view_rect, .. } = &self.meta_data {
                    PartMetaData::Raster {
                        view_rect: *view_rect,
                        tint: Some(tint),
                    }
                } else {
                    return Err("Applied Raster update to Vector part".to_owned());
                };
            };
            if let Some(view_rect) = new_view_rect {
                self.meta_data = if let PartMetaData::Raster { tint, .. } = &self.meta_data {
                    PartMetaData::Raster {
                        view_rect,
                        tint: *tint,
                    }
                } else {
                    return Err("Applied Raster update to Vector part".to_owned());
                };
                self.mark_dirty();
                update_bounds = true;
            };
            if let Some(coloring) = new_coloring {
                self.meta_data = if let PartMetaData::Vector { .. } = &self.meta_data {
                    PartMetaData::Vector {
                        color: Some(coloring),
                    }
                } else {
                    return Err("Applied Vector update to Raster part".to_owned());
                };
            };
        };
        Ok(update_bounds)
    }
}

#[derive(Clone, Debug)]
pub struct Entity {
    active: bool,
    dirty: bool,
    children: Vec<Uuid>,
    depth: u32,
    id: Uuid,
    morph_index: f32,
    name: String,
    parent: Uuid,
    parts: HashMap<Uuid, Part>,
    transform: Transform2F,
    tweens: Vec<PropertyTween>,
    bounding_box: RectF,
}

impl Entity {
    pub fn new(
        id: Uuid,
        depth: u32,
        name: &str,
        parent: Uuid,
        parts: HashMap<Uuid, Part>,
        transform: Transform2F,
        morph_index: f32,
    ) -> Self {
        Self {
            active: true,
            dirty: true,
            children: vec![],
            depth,
            id,
            name: name.to_owned(),
            parent,
            parts,
            transform,
            tweens: vec![],
            morph_index,
            bounding_box: RectF::default(),
        }
    }

    pub fn create_root(id: Uuid) -> Self {
        Self::new(
            id,
            0,
            "Root",
            id,
            HashMap::new(),
            Transform2F::default(),
            0.0,
        )
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn bounds(&self) -> &RectF {
        &self.bounding_box
    }

    pub fn recompute_bounds(
        &mut self,
        transform: &Transform2F,
        library: &HashMap<Uuid, DisplayLibraryItem>,
    ) -> RectF {
        let morph_index = self.morph_index;
        self.bounding_box = self
            .parts
            .iter_mut()
            .filter(|(_, part)| part.active())
            .map(|(_, part)| part.recompute_bounds(transform, morph_index, library))
            .reduce(|a, b| a.union_rect(b))
            .unwrap_or(RectF::default());
        self.bounding_box
    }

    #[inline]
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    #[inline]
    pub fn parent(&self) -> &Uuid {
        &self.parent
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn active(&self) -> bool {
        self.active
    }

    #[inline]
    pub fn depth(&self) -> u32 {
        self.depth
    }

    #[inline]
    pub fn transform(&self) -> &Transform2F {
        &self.transform
    }

    #[inline]
    pub fn morph_index(&self) -> f32 {
        self.morph_index
    }

    #[inline]
    pub fn parts(&self) -> impl Iterator<Item = &Part> {
        self.parts.iter().map(|(_, part)| part)
    }

    #[inline]
    pub fn parts_with_id(&self) -> impl Iterator<Item = (&Uuid, &Part)> {
        self.parts.iter()
    }

    pub fn get_part(&self, part_id: &Uuid) -> Option<&Part> {
        self.parts.get(part_id)
    }

    pub fn get_part_mut(&mut self, part_id: &Uuid) -> Option<&mut Part> {
        self.parts.get_mut(part_id)
    }

    #[inline]
    pub fn add_part(&mut self, part_id: &Uuid, mut part: Part) {
        part.mark_dirty();
        self.parts.insert(*part_id, part);
        self.mark_dirty();
    }

    #[inline]
    pub fn remove_part(&mut self, part_id: &Uuid) -> Option<Part> {
        let removed = self.parts.remove(part_id);
        // State wise, it's more technically accurate to mark dirty after removing the part.
        self.mark_dirty();
        removed
    }

    #[inline]
    pub fn children(&self) -> &Vec<Uuid> {
        &self.children
    }

    #[inline]
    pub fn add_child(&mut self, child: Uuid) {
        self.children.push(child);
    }

    #[inline]
    pub fn remove_child(&mut self, child: &Uuid) {
        self.children.retain(|elem| elem != child);
    }

    pub fn add_tweens(
        &mut self,
        entity_update_definition: &EntityUpdateDefinition,
        duration_seconds: f32,
        library: &HashMap<Uuid, DisplayLibraryItem>,
    ) -> Result<(), String> {
        for entity_update in entity_update_definition.entity_updates() {
            match entity_update {
                EntityUpdatePayload::Transform { easing, transform } => {
                    let tween = PropertyTween::new_transform(
                        ScaleRotationTranslation::from_transform(&self.transform),
                        *transform,
                        duration_seconds,
                        *easing,
                    );
                    self.tweens.push(tween);
                }
                EntityUpdatePayload::MorphIndex {
                    easing,
                    morph_index,
                } => {
                    let tween = PropertyTween::new_morph_index(
                        self.morph_index,
                        *morph_index,
                        duration_seconds,
                        *easing,
                    );
                    self.tweens.push(tween);
                }
            }
        }
        for part_update in entity_update_definition.part_updates() {
            if let Some(part) = self.parts.get_mut(part_update.part_id()) {
                if let Some(library_item) = library.get(part.item_id()) {
                    part.add_tween(library_item, part_update, duration_seconds)?;
                }
            }
        }
        Ok(())
    }

    // TODO: Research frames per elapsed time for tweens
    pub fn update_tweens(&mut self, elapsed: f32) -> Result<(), String> {
        if !self.tweens.is_empty() {
            let mut transform_update = Transform2F::default();
            let mut morph_update = 0.0;
            let mut morph_updates_accumulated = 0.0;
            for tween in self.tweens.iter_mut() {
                match tween.update(elapsed) {
                    PropertyTweenUpdate::Transform(transform) => {
                        // update accumulator
                        transform_update = transform * transform_update;
                        // apply accumulator
                        self.transform = transform_update;
                    }
                    PropertyTweenUpdate::Morph(morph_index) => {
                        // update accumulator
                        morph_update += morph_index;
                        morph_updates_accumulated += 1.0;
                        // apply accumulator
                        self.morph_index = morph_update / morph_updates_accumulated;
                    }
                    _ => (),
                }
            }
            self.mark_dirty();
            // Pessemistically mark all parts as dirty if the entity updated
            self.parts
                .iter_mut()
                .for_each(|(_, part)| part.mark_dirty());
            self.tweens.retain(|tween| !tween.is_complete());
        }
        let mut update_bounds = false;
        for (_, part) in self.parts.iter_mut() {
            update_bounds |= part.update_tweens(elapsed)?;
        }
        if update_bounds {
            self.mark_dirty();
        }
        Ok(())
    }
}
