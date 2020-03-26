#![deny(clippy::all)]

use super::rendering::{Bitmap, ColorUDef, Coloring, Shape};
use super::tween::Easing;
use super::types::{transform_des, transform_ser, ScaleRotationTranslation, Vector2FDef};
use core::cmp::min;
use pathfinder_color::ColorU;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

    pub fn jump_to_frame(&mut self, frame: u32) -> Result<(usize, u32), String> {
        unimplemented!()
    }

    // TODO : this should return a tuple of (usize, u32) where the first is the action_index and the second is the new frame index
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
pub enum PartDefinition {
    Vector {
        item_id: Uuid,
        #[serde(serialize_with = "transform_ser", deserialize_with = "transform_des")]
        transform: Transform2F,
    },
    Bitmap {
        item_id: Uuid,
        #[serde(serialize_with = "transform_ser", deserialize_with = "transform_des")]
        transform: Transform2F,
        view_rect: RectPoints,
    },
}

impl PartDefinition {
    pub fn item_id(&self) -> &Uuid {
        match self {
            PartDefinition::Vector { item_id, .. } => item_id,
            PartDefinition::Bitmap { item_id, .. } => item_id,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PartUpdateDefinition {
    Vector {
        color: Option<Coloring>,
        easing: Easing,
        //TODO: Gradients
        item_id: Uuid,
        transform: Option<ScaleRotationTranslation>,
    },
    Bitmap {
        #[serde(
            serialize_with = "option_color_ser",
            deserialize_with = "option_color_des"
        )]
        tint: Option<ColorU>,
        easing: Easing,
        item_id: Uuid,
        transform: Option<ScaleRotationTranslation>,
        view_rect: Option<RectPoints>,
    },
}

impl PartUpdateDefinition {
    pub fn item_id(&self) -> &Uuid {
        match self {
            PartUpdateDefinition::Vector { item_id, .. } => item_id,
            PartUpdateDefinition::Bitmap { item_id, .. } => item_id,
        }
    }
}

//TODO: since these vecs are immutable, replace with Box<[T]> (into_boxed_slice())
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
    pub duration_frames: u16,
    pub easing: Option<Easing>,
    pub id: Uuid,
    pub part_updates: Vec<PartUpdateDefinition>,
    pub transform: Option<ScaleRotationTranslation>,
    pub morph_index: Option<f32>,
}

//TODO: additional actions: Text, Scripts, Fonts, AddPart, RemovePart
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Action {
    CreateRoot(Uuid),
    SetBackground {
        #[serde(with = "ColorUDef")]
        color: ColorU,
    },
    EndInitialization,
    Label(String),
    DefineShape {
        id: Uuid,
        shape: Shape,
    },
    LoadBitmap {
        id: Uuid,
        bitmap: Bitmap,
    },
    AddEntity(EntityDefinition),
    UpdateEntity(EntityUpdateDefinition),
    RemoveEntity(Uuid),
    PresentFrame(u32, u32), //TODO: if frames have set indexes, then how would it be possible to load in additional frames? Clip ID?
    Quit,
}

#[derive(Serialize, Deserialize)]
struct ColorWrapper(#[serde(with = "ColorUDef")] ColorU);

fn option_color_des<'de, D>(deserializer: D) -> Result<Option<ColorU>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::deserialize(deserializer) {
        Ok(option) => match option {
            Some(ColorWrapper(c)) => Ok(Some(c)),
            None => Ok(None),
        },
        Err(err) => Err(err),
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn option_color_ser<S>(u: &Option<ColorU>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match u {
        Some(ref color) => serializer.serialize_some(&ColorWrapper(*color)),
        None => serializer.serialize_none(),
    }
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
