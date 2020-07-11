#![deny(clippy::all)]

use super::tween::Easing;
use super::types::{
    basic::{transform_des, transform_ser, Bitmap, ScaleRotationTranslation, Vector2FDef},
    coloring::Coloring,
    shapes::Shape,
};
use crate::ecs::resources::{QuadTreeLayer, QuadTreeLayerOptions};
use core::cmp::min;
use palette::LinSrgb;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use streaming_iterator::StreamingIterator;
use uuid::Uuid;

pub struct ActionList {
    actions: Vec<Action>,
    //TODO: move frame_index here
    action_index: usize,
    labels: HashMap<String, usize>,
    load_more: Box<dyn Fn() -> Option<Vec<Action>>>,
}

impl ActionList {
    pub fn new(
        load_more: Box<dyn Fn() -> Option<Vec<Action>>>,
        initial_vec: Option<&Vec<Action>>,
    ) -> ActionList {
        let initial_vec = match initial_vec {
            Some(vec) => vec.to_vec(),
            None => vec![],
        };
        ActionList {
            actions: initial_vec,
            labels: HashMap::new(),
            action_index: 0,
            load_more,
        }
    }

    pub fn current_index(&self) -> usize {
        self.action_index
    }

    pub fn jump_to_label(&mut self, label: &str) -> Result<(usize, u32), String> {
        let new_index = match self.labels.get(label) {
            Some(index) => *index,
            None => {
                let mut index: Option<usize> = None;
                for i in self.action_index..self.actions.len() {
                    if let Action::Label(name) = &self.actions[i] {
                        self.labels.insert(name.clone(), i);
                        if name == label {
                            index = Some(i);
                            break;
                        }
                    }
                }
                match index {
                    Some(i) => i,
                    None => {
                        return Err(format!(
                            "Could not find label {} in any loaded actions",
                            label
                        ))
                    }
                }
            }
        };
        self.action_index = new_index;
        let mut search = new_index;
        loop {
            if search == 0 {
                return Ok((new_index, 0));
            }
            search -= 1; //new_index will be Action::Label
            let action = self.actions.get(search);
            match action {
                Some(Action::PresentFrame(start, count)) => return Ok((new_index, start + count)),
                Some(Action::EndInitialization) => return Ok((new_index, 0)),
                _ => (),
            }
        }
    }

    pub fn back(&mut self) {
        match self.actions.get(self.action_index - 1) {
            Some(Action::EndInitialization) => (),
            None => (),
            _ => self.action_index -= 1,
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut Action> {
        self.actions.get_mut(self.action_index)
    }
}

impl StreamingIterator for ActionList {
    type Item = Action;

    fn advance(&mut self) {
        if let Some(mut more) = (self.load_more)() {
            self.actions.append(&mut more);
        }
        self.action_index = min(self.action_index + 1, self.actions.len() - 1);
        if let Some(Action::Label(name)) = self.actions.get(self.action_index) {
            self.labels.insert(name.clone(), self.action_index);
        }
    }

    fn get(&self) -> Option<&Self::Item> {
        self.actions.get(self.action_index)
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct RectPoints {
    #[serde(with = "Vector2FDef")]
    pub origin: Vector2F,
    #[serde(with = "Vector2FDef")]
    pub lower_right: Vector2F,
}

impl RectPoints {
    pub fn from_rect(rect: &RectF) -> RectPoints {
        RectPoints {
            origin: rect.origin(),
            lower_right: rect.lower_right(),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PartDefinition {
    part_id: Uuid,
    item_id: Uuid,
    transform: ScaleRotationTranslation,
    payload: Vec<PartDefinitionPayload>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PartDefinitionPayload {
    Coloring(Coloring),
    ViewRect(RectPoints),
}

impl PartDefinition {
    pub fn new(
        part_id: Uuid,
        item_id: Uuid,
        transform: ScaleRotationTranslation,
        payload: Vec<PartDefinitionPayload>,
    ) -> Self {
        Self {
            part_id,
            item_id,
            transform,
            payload,
        }
    }

    pub fn part_id(&self) -> &Uuid {
        &self.part_id
    }

    pub fn item_id(&self) -> &Uuid {
        &self.item_id
    }

    pub fn transform(&self) -> Transform2F {
        Transform2F::from_scale_rotation_translation(
            self.transform.scale,
            self.transform.theta,
            self.transform.translation,
        )
    }

    pub fn payload(&self) -> impl Iterator<Item = &PartDefinitionPayload> {
        self.payload.iter()
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PartUpdateDefinition {
    part_id: Uuid,
    easing: Easing,
    payload: PartUpdatePayload,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PartUpdatePayload {
    Transform(ScaleRotationTranslation),
    Coloring(Coloring),
    ViewRect(RectPoints),
}

impl PartUpdatePayload {
    pub fn from_scale_rotation_translation(
        scale_rotation_translation: ScaleRotationTranslation,
    ) -> Self {
        Self::Transform(scale_rotation_translation)
    }

    pub fn from_transform(transform: &Transform2F) -> Self {
        Self::Transform(ScaleRotationTranslation::from_transform(transform))
    }

    pub fn from_coloring(coloring: Coloring) -> Self {
        Self::Coloring(coloring)
    }

    pub fn from_view_rect_points(rect_points: RectPoints) -> Self {
        Self::ViewRect(rect_points)
    }

    pub fn from_view_rect(rect: &RectF) -> Self {
        Self::ViewRect(RectPoints {
            origin: rect.origin(),
            lower_right: rect.lower_right(),
        })
    }
}

impl PartUpdateDefinition {
    pub fn new(part_id: Uuid, easing: Easing, payload: PartUpdatePayload) -> Self {
        Self {
            part_id,
            easing,
            payload,
        }
    }

    pub fn part_id(&self) -> &Uuid {
        &self.part_id
    }

    pub fn easing(&self) -> Easing {
        self.easing
    }

    pub fn payload(&self) -> &PartUpdatePayload {
        &self.payload
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct EntityDefinition {
    pub depth: u32,
    pub id: Uuid,
    pub name: String,
    pub parent: Option<Uuid>,
    pub parts: Vec<PartDefinition>,
    #[serde(serialize_with = "transform_ser", deserialize_with = "transform_des")]
    pub transform: Transform2F,
    pub morph_index: f32,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct EntityUpdateDefinition {
    duration_frames: u16,
    id: Uuid,
    part_updates: Vec<PartUpdateDefinition>,
    entity_updates: Vec<EntityUpdatePayload>,
}

impl EntityUpdateDefinition {
    pub fn new(
        id: Uuid,
        duration_frames: u16,
        part_updates: Vec<PartUpdateDefinition>,
        entity_updates: Vec<EntityUpdatePayload>,
    ) -> Self {
        Self {
            id,
            duration_frames,
            part_updates,
            entity_updates,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn duration_frames(&self) -> u16 {
        self.duration_frames
    }

    pub fn part_updates(&self) -> impl Iterator<Item = &PartUpdateDefinition> {
        self.part_updates.iter()
    }

    pub fn entity_updates(&self) -> impl Iterator<Item = &EntityUpdatePayload> {
        self.entity_updates.iter()
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum EntityUpdatePayload {
    Transform {
        easing: Easing,
        transform: ScaleRotationTranslation,
    },
    MorphIndex {
        easing: Easing,
        morph_index: f32,
    },
}

impl EntityUpdatePayload {
    pub fn from_morph_index(morph_index: f32, easing: Easing) -> Self {
        Self::MorphIndex {
            morph_index,
            easing,
        }
    }

    pub fn from_scale_rotation_translation(
        scale_rotation_translation: ScaleRotationTranslation,
        easing: Easing,
    ) -> Self {
        Self::Transform {
            transform: scale_rotation_translation,
            easing,
        }
    }

    pub fn from_transform(transform: &Transform2F, easing: Easing) -> Self {
        Self::Transform {
            transform: ScaleRotationTranslation::from_transform(&transform),
            easing,
        }
    }
}
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ContainerCreationDefintition {
    id: Uuid,
    parent: Uuid,
    properties: Vec<ContainerCreationProperty>,
}

impl ContainerCreationDefintition {
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn parent(&self) -> &Uuid {
        &self.parent
    }

    pub fn properties(&self) -> &Vec<ContainerCreationProperty> {
        &self.properties
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum ContainerCreationProperty {
    Transform(ScaleRotationTranslation),
    MorphIndex(f32),
    Coloring(Coloring),
    ViewRect(RectPoints),
    Display(Uuid),
    Layer(QuadTreeLayer),
    Order(i8),
    Bounds(BoundsKindDefinition),
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ContainerUpdateDefintition {
    id: Uuid,
    properties: Vec<ContainerUpdateProperty>,
}

impl ContainerUpdateDefintition {
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn properties(&self) -> &Vec<ContainerUpdateProperty> {
        &self.properties
    }
}
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum ContainerUpdateProperty {
    Transform(ScaleRotationTranslation, Easing, u32),
    MorphIndex(f32, Easing, u32),
    Coloring(Coloring, Easing, u32),
    ViewRect(RectPoints, Easing, u32),
    Order(i8, Easing, u32),
    Display(Uuid),
    RemoveDisplay,
    Parent(Uuid),
    AddToLayer(QuadTreeLayer),
    RemoveFromLayer(QuadTreeLayer),
    Bounds(BoundsKindDefinition),
    RemoveBounds,
}
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum BoundsKindDefinition {
    Display,
    Defined(RectPoints),
}

//TODO: additional actions: Text, Scripts, Fonts
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Action {
    CreateRoot(Uuid),
    AddQuadTreeLayer(QuadTreeLayer, RectPoints, QuadTreeLayerOptions),
    SetBackground { color: LinSrgb },
    EndInitialization,
    Label(String),
    DefineShape { id: Uuid, shape: Shape },
    LoadBitmap { id: Uuid, bitmap: Bitmap },
    CreateContainer(ContainerCreationDefintition),
    UpdateContainer(ContainerUpdateDefintition),
    RemoveContainer(Uuid, bool),
    PresentFrame(u32, u32), //TODO: if frames have set indexes, then how would it be possible to load in additional frames? Clip ID?
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_advances_actions() {
        let actions = vec![Action::PresentFrame(1, 1)];
        let mut action_list = ActionList::new(Box::new(|| None), Some(&actions));
        assert_eq!(action_list.current_index(), 0);
        assert_eq!(action_list.actions.len(), 1);
        action_list.get().expect("Did not return expected action");
        assert_eq!(action_list.current_index(), 0);
        action_list.advance();
        assert_eq!(action_list.current_index(), 0); //Does not advance past the end of the list
        action_list.get().expect("Did not return expected action");
    }

    #[test]
    fn it_loads_more() {
        let actions = vec![Action::PresentFrame(1, 1)];
        let mut action_list = ActionList::new(
            Box::new(|| {
                Some(vec![
                    Action::PresentFrame(2, 1),
                    Action::PresentFrame(3, 1),
                    Action::PresentFrame(4, 1),
                ])
            }),
            Some(&actions),
        );
        assert_eq!(action_list.current_index(), 0);
        assert_eq!(action_list.actions.len(), 1);
        action_list.advance();
        assert_eq!(action_list.actions.len(), 4);
    }

    #[test]
    fn it_jumps() {
        let actions = vec![
            Action::PresentFrame(1, 1),
            Action::Label(String::from("label_1")),
            Action::PresentFrame(2, 1),
            Action::PresentFrame(3, 1),
            Action::Label(String::from("label_2")),
            Action::PresentFrame(4, 1),
        ];
        let mut action_list = ActionList::new(Box::new(|| None), Some(&actions));
        action_list.advance();
        action_list.advance();
        action_list.advance();
        assert_eq!(action_list.current_index(), 3);
        assert_eq!(action_list.labels.len(), 1);
        let result = action_list.jump_to_label("label_1").unwrap();
        assert_eq!(result, (1, 2));
        assert_eq!(action_list.current_index(), 1);
        let result = action_list.jump_to_label("label_2").unwrap();
        assert_eq!(result, (4, 4));
        assert_eq!(action_list.labels.len(), 2);
        assert_eq!(action_list.current_index(), 4);
    }
}
