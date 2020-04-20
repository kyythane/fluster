use super::basic::ScaleRotationTranslation;
use super::shapes::{Coloring, Shape};
use crate::actions::{EntityUpdateDefinition, PartUpdateDefinition, RectPoints};
use crate::tween::{PropertyTween, PropertyTweenUpdate, Tween};
use pathfinder_color::ColorU;
use pathfinder_content::pattern::Pattern;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use std::collections::HashMap;
use std::mem;
use uuid::Uuid;

#[derive(Clone, PartialEq, Debug)]
pub enum DisplayLibraryItem {
    Vector(Shape),
    Raster(Pattern),
}

#[derive(Clone, PartialEq, Debug)]
pub enum Part {
    Vector {
        item_id: Uuid,
        transform: Transform2F,
        color: Option<Coloring>,
    },
    Raster {
        item_id: Uuid,
        view_rect: RectF,
        transform: Transform2F,
        tint: Option<ColorU>,
    },
}

impl Part {
    pub fn new_vector(item_id: Uuid, transform: Transform2F, color: Option<Coloring>) -> Part {
        Part::Vector {
            item_id,
            transform,
            color,
        }
    }

    pub fn new_raster(
        item_id: Uuid,
        view_rect: RectF,
        transform: Transform2F,
        tint: Option<ColorU>,
    ) -> Part {
        Part::Raster {
            item_id,
            view_rect,
            transform,
            tint,
        }
    }

    pub fn item_id(&self) -> &Uuid {
        match self {
            Part::Vector { item_id, .. } => item_id,
            Part::Raster { item_id, .. } => item_id,
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
    children: Vec<Uuid>,
    depth: u32,
    id: Uuid,
    morph_index: f32,
    name: String,
    parent: Uuid,
    parts: Vec<Part>,
    transform: Transform2F,
    tweens: HashMap<Uuid, Vec<PropertyTween>>,
}

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
    ) -> Entity {
        Entity {
            active: true,
            children: vec![],
            depth,
            id,
            name: name.to_owned(),
            parent,
            parts,
            transform,
            tweens: HashMap::new(),
            morph_index,
        }
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
        Ok(())
    }

    pub fn update_tweens(&mut self, elapsed: f32) {
        if self.tweens.is_empty() {
            return;
        }
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
                    },
                    Part::Raster {
                        item_id,
                        transform,
                        view_rect,
                        tint,
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
                    },
                };
                mem::replace(part, new_part);
                tweens.retain(|tween| !tween.is_complete());
            }
        }
    }
}
