use super::tween::Easing;
use super::types::{
    basic::{Bitmap, ScaleRotationTranslation, Vector2FDef},
    coloring::Coloring,
    shapes::Shape,
};
use crate::{
    ecs::resources::{QuadTreeLayer, QuadTreeLayerOptions},
    types::{
        basic::{ContainerId, LibraryId},
        coloring::ColorSpace,
    },
};
use core::cmp::min;
use palette::LinSrgb;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::Vector2F;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use streaming_iterator::StreamingIterator;

pub enum FrameAdvanceResult {
    NextFrame(u32),
    PresentEnd(u32),
    NotInPresent,
}

pub struct ActionList {
    actions: Vec<Action>,
    frame_index: u32,
    action_index: usize,
    labels: HashMap<String, usize>,
    load_more: Box<dyn Fn() -> Option<Vec<Action>>>,
}

impl ActionList {
    pub fn new(
        load_more: Box<dyn Fn() -> Option<Vec<Action>>>,
        initial_vec: Option<&Vec<Action>>,
    ) -> Self {
        let initial_vec = match initial_vec {
            Some(vec) => vec.to_vec(),
            None => vec![],
        };
        Self {
            actions: initial_vec,
            labels: HashMap::new(),
            frame_index: 0,
            action_index: 0,
            load_more,
        }
    }

    pub fn action_index(&self) -> usize {
        self.action_index
    }

    pub fn frame_index(&self) -> u32 {
        self.frame_index
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
            match self.actions.get(search) {
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

    pub fn advance_frame(&mut self, num_frames: u32) -> FrameAdvanceResult {
        if let Some(Action::PresentFrame(start, count)) = self.actions.get(self.action_index) {
            let max = *start + *count;
            if self.frame_index > max {
                FrameAdvanceResult::PresentEnd(max)
            } else {
                self.frame_index = (self.frame_index.max(*start) + num_frames).min(max);
                if self.frame_index == max {
                    FrameAdvanceResult::PresentEnd(self.frame_index)
                } else {
                    FrameAdvanceResult::NextFrame(self.frame_index)
                }
            }
        } else {
            FrameAdvanceResult::NotInPresent
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
pub struct ContainerCreationDefintition {
    id: ContainerId,
    parent: ContainerId,
    properties: Vec<ContainerCreationProperty>,
}

impl ContainerCreationDefintition {
    pub fn new(
        parent: ContainerId,
        id: ContainerId,
        properties: Vec<ContainerCreationProperty>,
    ) -> Self {
        Self {
            id,
            parent,
            properties,
        }
    }

    pub fn id(&self) -> &ContainerId {
        &self.id
    }

    pub fn parent(&self) -> &ContainerId {
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
    Display(LibraryId),
    Layer(QuadTreeLayer),
    Order(i8),
    Bounds(BoundsKindDefinition),
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ContainerUpdateDefintition {
    id: ContainerId,
    properties: Vec<ContainerUpdateProperty>,
}

impl ContainerUpdateDefintition {
    pub fn new(id: ContainerId, properties: Vec<ContainerUpdateProperty>) -> Self {
        Self { id, properties }
    }

    pub fn id(&self) -> &ContainerId {
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
    Coloring(Coloring, ColorSpace, Easing, u32),
    ViewRect(RectPoints, Easing, u32),
    Order(i8, Easing, u32),
    Display(LibraryId),
    RemoveDisplay,
    Parent(ContainerId),
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
    CreateRoot(ContainerId),
    AddQuadTreeLayer(QuadTreeLayer, RectPoints, QuadTreeLayerOptions),
    SetBackground { color: LinSrgb },
    EndInitialization,
    Label(String),
    DefineShape { id: LibraryId, shape: Shape },
    LoadBitmap { id: LibraryId, bitmap: Bitmap },
    CreateContainer(ContainerCreationDefintition),
    UpdateContainer(ContainerUpdateDefintition),
    RemoveContainer(ContainerId, bool),
    PresentFrame(u32, u32), //TODO: if frames have set indexes, then how would it be possible to load in additional frames? Clip ID?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_advances_actions() {
        let actions = vec![Action::PresentFrame(1, 1)];
        let mut action_list = ActionList::new(Box::new(|| None), Some(&actions));
        assert_eq!(action_list.action_index(), 0);
        assert_eq!(action_list.actions.len(), 1);
        action_list.get().expect("Did not return expected action");
        assert_eq!(action_list.action_index(), 0);
        action_list.advance();
        assert_eq!(action_list.action_index(), 0); //Does not advance past the end of the list
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
        assert_eq!(action_list.action_index(), 0);
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
        assert_eq!(action_list.action_index(), 3);
        assert_eq!(action_list.labels.len(), 1);
        let result = action_list.jump_to_label("label_1").unwrap();
        assert_eq!(result, (1, 2));
        assert_eq!(action_list.action_index(), 1);
        let result = action_list.jump_to_label("label_2").unwrap();
        assert_eq!(result, (4, 4));
        assert_eq!(action_list.labels.len(), 2);
        assert_eq!(action_list.action_index(), 4);
    }
}
