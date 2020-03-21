#![deny(clippy::all)]

use super::rendering::{Bitmap, ColorUDef, Shape, Vector2FDef};
use core::cmp::min;
use pathfinder_color::ColorU;
use pathfinder_geometry::vector::Vector2F;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use streaming_iterator::StreamingIterator;
use uuid::Uuid;

pub struct ActionList {
    actions: Vec<Action>,
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
    pub fn jump_to_label(&mut self, label: &str) -> Result<usize, String> {
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
        Ok(new_index)
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

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct ScaleRotationTranslation {
    #[serde(with = "Vector2FDef")]
    pub scale: Vector2F,
    pub theta: f32,
    #[serde(with = "Vector2FDef")]
    pub translation: Vector2F,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PartDefinition {
    Vector {
        item_id: Uuid,
        transform: ScaleRotationTranslation,
    },
    Bitmap {
        item_id: Uuid,
        view_rect: RectPoints,
        transform: ScaleRotationTranslation,
    },
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PartUpdateDefinition {
    Vector {
        item_id: Uuid,
        transform: Option<ScaleRotationTranslation>,
        #[serde(
            serialize_with = "option_color_ser",
            deserialize_with = "option_color_des"
        )]
        color: Option<ColorU>,
    },
    Bitmap {
        item_id: Uuid,
        transform: Option<ScaleRotationTranslation>,
        #[serde(
            serialize_with = "option_color_ser",
            deserialize_with = "option_color_des"
        )]
        color: Option<ColorU>,
    },
}

//TODO: Fill in additional types or convert to struct
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum EntityDefinition {
    SimpleEntity {
        id: Uuid,
        name: String,
        transform: ScaleRotationTranslation,
        depth: u32,
        parts: Vec<PartDefinition>,
        parent: Option<Uuid>,
    },
}

//TODO: additional actions: Text, Scripts, Fonts
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
    UpdateEntity {
        id: Uuid,
        duration_frames: u16,
        transform: Option<ScaleRotationTranslation>,
        part_updates: Vec<PartUpdateDefinition>,
        #[serde(
            serialize_with = "option_color_ser",
            deserialize_with = "option_color_des"
        )]
        color: Option<ColorU>,
        //TODO: morph shapes
    },
    RemoveEntity {
        id: Uuid,
    },
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
                    Action::PresentFrame(1, 1),
                    Action::PresentFrame(1, 1),
                    Action::PresentFrame(1, 1),
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
            Action::PresentFrame(1, 1),
            Action::PresentFrame(1, 1),
            Action::Label(String::from("label_2")),
            Action::PresentFrame(1, 1),
        ];
        let mut action_list = ActionList::new(Box::new(|| None), Some(&actions));
        action_list.advance();
        action_list.advance();
        action_list.advance();
        assert_eq!(action_list.current_index(), 3);
        assert_eq!(action_list.labels.len(), 1);
        action_list.jump_to_label("label_1").unwrap();
        assert_eq!(action_list.current_index(), 1);
        action_list.jump_to_label("label_2").unwrap();
        assert_eq!(action_list.labels.len(), 2);
        assert_eq!(action_list.current_index(), 4);
    }
}
