use super::basic::ScaleRotationTranslation;
use super::shapes::{Coloring, Shape};
use crate::actions::{EntityUpdateDefinition, PartUpdateDefinition, RectPoints};
use crate::tween::{PropertyTween, PropertyTweenUpdate, Tween};
use pathfinder_color::ColorU;
use pathfinder_content::pattern::Pattern;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::{transform2d::Transform2F, vector::Vector2F};
use reduce::Reduce;
use std::collections::HashMap;
use std::mem;
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

#[derive(Clone, PartialEq, Debug)]
pub enum Part {
    Vector {
        item_id: Uuid,
        transform: Transform2F,
        bounding_box: RectF,
        color: Option<Coloring>,
    },
    Raster {
        item_id: Uuid,
        view_rect: RectF,
        transform: Transform2F,
        bounding_box: RectF,
        tint: Option<ColorU>,
    },
}

impl Part {
    pub fn new_vector(item_id: Uuid, transform: Transform2F, color: Option<Coloring>) -> Part {
        Self::Vector {
            item_id,
            transform,
            bounding_box: RectF::default(),
            color,
        }
    }

    pub fn new_raster(
        item_id: Uuid,
        view_rect: RectF,
        transform: Transform2F,
        tint: Option<ColorU>,
    ) -> Part {
        Self::Raster {
            item_id,
            view_rect,
            bounding_box: RectF::default(),
            transform,
            tint,
        }
    }

    pub fn recompute_bounds(
        &mut self,
        world_transform: &Transform2F,
        morph_percent: f32,
        library: &HashMap<Uuid, DisplayLibraryItem>,
    ) -> RectF {
        let new_self = match self {
            Self::Vector {
                item_id,
                transform,
                color,
                ..
            } => {
                let bounding_box = library
                    .get(item_id)
                    .unwrap()
                    .compute_bounding(&(*world_transform * *transform), morph_percent);
                Self::Vector {
                    item_id: *item_id,
                    transform: *transform,
                    color: color.clone(),
                    bounding_box,
                }
            }
            Self::Raster {
                item_id,
                view_rect,
                transform,
                tint,
                ..
            } => {
                let transform = *world_transform * *transform;
                let o = transform * Vector2F::default();
                let lr = transform * view_rect.size();
                let bounding_box = RectF::from_points(o.min(lr), o.max(lr));
                Self::Raster {
                    item_id: *item_id,
                    view_rect: *view_rect,
                    transform,
                    tint: *tint,
                    bounding_box,
                }
            }
        };
        mem::replace(self, new_self);
        *self.bounds()
    }

    pub fn bounds(&self) -> &RectF {
        match self {
            Self::Vector { bounding_box, .. } | Self::Raster { bounding_box, .. } => bounding_box,
        }
    }

    pub fn item_id(&self) -> &Uuid {
        match self {
            Self::Vector { item_id, .. } | Self::Raster { item_id, .. } => item_id,
        }
    }

    pub fn create_tween(
        &self,
        library_item: &DisplayLibraryItem,
        part_update: &PartUpdateDefinition,
        duration_seconds: f32,
    ) -> Result<Vec<PropertyTween>, String> {
        let mut tweens: Vec<PropertyTween> = vec![];
        match part_update {
            PartUpdateDefinition::Raster {
                tint: end_tint,
                easing,
                transform: end_transform,
                view_rect: end_view_rect,
                ..
            } => {
                if let Part::Raster {
                    transform: start_transform,
                    tint: start_tint,
                    view_rect: start_view_rect,
                    ..
                } = self
                {
                    if let Some(end_tint) = end_tint {
                        if let Some(start_tint) = start_tint {
                            tweens.push(PropertyTween::new_color(
                                *start_tint,
                                *end_tint,
                                duration_seconds,
                                *easing,
                            ));
                        } else {
                            tweens.push(PropertyTween::new_color(
                                ColorU::white(),
                                *end_tint,
                                duration_seconds,
                                *easing,
                            ));
                        }
                    }
                    if let Some(end_transform) = end_transform {
                        tweens.push(PropertyTween::new_transform(
                            ScaleRotationTranslation::from_transform(start_transform),
                            *end_transform,
                            duration_seconds,
                            *easing,
                        ));
                    }
                    if let Some(end_view_rect) = end_view_rect {
                        tweens.push(PropertyTween::new_view_rect(
                            RectPoints::from_rect(start_view_rect),
                            *end_view_rect,
                            duration_seconds,
                            *easing,
                        ));
                    }
                } else {
                    return Err(format!(
                        "Tried to apply Bitmap update to a Vector part {}",
                        self.item_id()
                    ));
                }
            }
            PartUpdateDefinition::Vector {
                color: end_color,
                easing,
                transform: end_transform,
                ..
            } => {
                if let Part::Vector {
                    transform: start_transform,
                    color: start_color,
                    ..
                } = self
                {
                    if let Some(end_color) = end_color {
                        if let Some(start_color) = start_color {
                            tweens.push(PropertyTween::new_coloring(
                                start_color.clone(),
                                end_color.clone(),
                                duration_seconds,
                                *easing,
                            ));
                        } else if let DisplayLibraryItem::Vector(shape) = library_item {
                            tweens.push(PropertyTween::new_coloring(
                                shape.color(),
                                end_color.clone(),
                                duration_seconds,
                                *easing,
                            ));
                        } else {
                            return Err(format!(
                                "Vector part {} references a Bitmap object",
                                self.item_id()
                            ));
                        }
                    }
                    if let Some(end_transform) = end_transform {
                        tweens.push(PropertyTween::new_transform(
                            ScaleRotationTranslation::from_transform(start_transform),
                            *end_transform,
                            duration_seconds,
                            *easing,
                        ));
                    }
                } else {
                    return Err(format!(
                        "Tried to apply Vector update to a Bitmap part {}",
                        self.item_id()
                    ));
                }
            }
        }
        Ok(tweens)
    }
}

//TODO: Bounding boxes and hit tests for mouse interactions
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
    parts: Vec<Part>,
    transform: Transform2F,
    tweens: HashMap<Uuid, Vec<PropertyTween>>,
    bounding_box: RectF,
}

// TODO: why did I do this?
impl PartialEq for Entity {
    //Tweens are ignored for the purpose of equality
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.children == other.children
            && self.depth == other.depth
            && self.name == other.name
            && self.parent == other.parent
            && self.parts == other.parts
            && self.transform == other.transform
    }
}

impl Entity {
    pub fn new(
        id: Uuid,
        depth: u32,
        name: &str,
        parent: Uuid,
        parts: Vec<Part>,
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
            tweens: HashMap::new(),
            morph_index,
            bounding_box: RectF::default(),
        }
    }

    pub fn create_root(id: Uuid) -> Self {
        Self::new(id, 0, "Root", id, vec![], Transform2F::default(), 0.0)
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
            .map(|part| part.recompute_bounds(transform, morph_index, library))
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
    pub fn parts(&self) -> &Vec<Part> {
        &self.parts
    }

    #[inline]
    pub fn add_part(&mut self, part: Part) {
        self.parts.push(part);
    }

    #[inline]
    pub fn remove_part(&mut self, item_id: &Uuid) {
        self.parts.retain(|part| part.item_id() != item_id);
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
        if let Some(easing) = entity_update_definition.easing {
            if let Some(end_transform) = &entity_update_definition.transform {
                let tween = PropertyTween::new_transform(
                    ScaleRotationTranslation::from_transform(&self.transform),
                    *end_transform,
                    duration_seconds,
                    easing,
                );
                match self.tweens.get_mut(&self.id) {
                    Some(tweens) => tweens.push(tween),
                    None => {
                        self.tweens.insert(self.id, vec![tween]);
                    }
                };
            }
            if let Some(end_morph) = &entity_update_definition.morph_index {
                let tween = PropertyTween::new_morph_index(
                    self.morph_index,
                    *end_morph,
                    duration_seconds,
                    easing,
                );
                match self.tweens.get_mut(&self.id) {
                    Some(tweens) => tweens.push(tween),
                    None => {
                        self.tweens.insert(self.id, vec![tween]);
                    }
                };
            }
        }
        for part_update in &entity_update_definition.part_updates {
            let update_item_id = part_update.item_id();
            if let Some(part) = self.parts.iter().find(|p| p.item_id() == update_item_id) {
                if let Some(library_item) = library.get(update_item_id) {
                    let tweens = part.create_tween(library_item, part_update, duration_seconds)?;
                    match self.tweens.get_mut(update_item_id) {
                        Some(existing_tweens) => existing_tweens.extend(tweens),
                        None => {
                            self.tweens.insert(*update_item_id, tweens);
                        }
                    };
                }
            }
        }
        self.mark_dirty();
        Ok(())
    }

    pub fn update_tweens(&mut self, elapsed: f32) {
        if self.tweens.is_empty() {
            return;
        }
        self.mark_dirty();
        for (key, tweens) in self.tweens.iter_mut() {
            if key == &self.id {
                for update in tweens.iter_mut().map(|tween| tween.update(elapsed)) {
                    match update {
                        PropertyTweenUpdate::Transform(transform) => {
                            self.transform = transform;
                        }
                        PropertyTweenUpdate::Morph(morph_index) => {
                            self.morph_index = morph_index;
                        }
                        _ => (),
                    }
                }
                tweens.retain(|tween| !tween.is_complete());
            } else if let Some(part) = self.parts.iter_mut().find(|p| p.item_id() == key) {
                let (new_transform, new_color, new_coloring, new_view_rect) = tweens
                    .iter_mut()
                    .map(|tween| {
                        let update = tween.update(elapsed);
                        match update {
                            PropertyTweenUpdate::Transform(transform) => {
                                (Some(transform), None, None, None)
                            }
                            PropertyTweenUpdate::Color(color) => (None, Some(color), None, None),
                            PropertyTweenUpdate::Coloring(coloring) => {
                                (None, None, Some(coloring), None)
                            }
                            PropertyTweenUpdate::ViewRect(view_rect) => {
                                (None, None, None, Some(view_rect))
                            }
                            PropertyTweenUpdate::Morph(_) => (None, None, None, None),
                        }
                    })
                    .fold((None, None, None, None), |acc, x| {
                        let t = if x.0.is_some() { x.0 } else { acc.0 };
                        let c = if x.1.is_some() { x.1 } else { acc.1 };
                        let cs = if x.2.is_some() { x.2 } else { acc.2 };
                        let v = if x.3.is_some() { x.3 } else { acc.3 };
                        (t, c, cs, v)
                    });
                let new_part = match part {
                    Part::Vector {
                        item_id,
                        transform,
                        color,
                        bounding_box,
                    } => Part::Vector {
                        item_id: *item_id,
                        transform: if let Some(new_transform) = new_transform {
                            new_transform
                        } else {
                            *transform
                        },
                        color: if new_coloring.is_some() {
                            new_coloring
                        } else {
                            color.take()
                        },
                        bounding_box: *bounding_box,
                    },
                    Part::Raster {
                        item_id,
                        transform,
                        view_rect,
                        tint,
                        bounding_box,
                    } => Part::Raster {
                        item_id: *item_id,
                        transform: if let Some(new_transform) = new_transform {
                            new_transform
                        } else {
                            *transform
                        },
                        tint: if new_color.is_some() {
                            new_color
                        } else {
                            *tint
                        },
                        view_rect: if let Some(new_view_rect) = new_view_rect {
                            new_view_rect
                        } else {
                            *view_rect
                        },
                        bounding_box: *bounding_box,
                    },
                };
                mem::replace(part, new_part);
                tweens.retain(|tween| !tween.is_complete());
            }
        }
    }
}
