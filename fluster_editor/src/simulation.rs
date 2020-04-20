#![deny(clippy::all)]
use fluster_core::rendering::{adjust_depth, RenderData};
use fluster_core::types::model::{DisplayLibraryItem, Entity};
use iced_native::{MouseCursor, Point};
use pathfinder_color::ColorU;
use pathfinder_geometry::transform2d::Transform2F;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::mem;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct StageState {
    background_color: ColorU,
    root_entity_id: Uuid,
    library: HashMap<Uuid, DisplayLibraryItem>,
    display_list: HashMap<Uuid, Entity>,
}

impl StageState {
    pub fn new(background_color: ColorU) -> Self {
        let root_entity_id = Uuid::new_v4();
        let mut display_list = HashMap::new();
        display_list.insert(root_entity_id, Entity::create_root(root_entity_id));
        Self {
            background_color,
            root_entity_id,
            library: HashMap::new(),
            display_list,
        }
    }
    pub fn background_color(&self) -> ColorU {
        self.background_color
    }
    pub fn library(&self) -> &HashMap<Uuid, DisplayLibraryItem> {
        &self.library
    }
}

impl StageState {
    pub fn compute_render_data(&self, timeline: &Timeline) -> RenderData {
        let mut nodes = VecDeque::new();
        let mut depth_list = BTreeMap::new();
        let mut world_space_transforms = HashMap::new();
        nodes.push_back(&self.root_entity_id);
        while let Some(entity_id) = nodes.pop_front() {
            if !timeline.can_show_entity(entity_id) {
                continue;
            }
            match self.display_list.get(entity_id) {
                Some(entity) => {
                    for child_id in entity.children() {
                        nodes.push_back(child_id);
                    }
                    let depth = adjust_depth(entity.depth(), &depth_list);
                    depth_list.insert(depth, entity);
                    if let Some(parent_transform) = world_space_transforms.get(entity.parent()) {
                        let parent_transform: Transform2F = *parent_transform;
                        world_space_transforms
                            .insert(*entity.id(), parent_transform * *entity.transform());
                    } else {
                        continue;
                    }
                }
                None => continue,
            }
        }
        RenderData::new(depth_list, world_space_transforms)
    }
}

impl Default for StageState {
    fn default() -> Self {
        Self::new(ColorU::white())
    }
}

#[derive(Debug, Clone)]
pub struct Timeline {
    layers: Vec<Layer>,
}

impl Timeline {
    pub fn can_show_entity(&self, id: &Uuid) -> bool {
        self.layers.iter().any(|layer| layer.can_show_entity(id))
    }

    pub fn set_layer_visible(&mut self, layer_index: usize, visible: bool) {
        if let Some(layer) = self.layers.get_mut(layer_index) {
            layer.set_visible(visible);
        }
    }

    pub fn get_layer_visible(&self, layer_index: usize) -> bool {
        match self.layers.get(layer_index) {
            Some(layer) => layer.visible,
            None => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Layer {
    frames: Vec<(Frame, (u32, u32))>,
    current_frame_index: usize,
    visible: bool,
}

impl Layer {
    pub fn can_show_entity(&self, id: &Uuid) -> bool {
        if !self.visible {
            return false;
        }
        self.contains_entity(id)
    }

    pub fn contains_entity(&self, id: &Uuid) -> bool {
        if let Some((frame, ..)) = self.frames.get(self.current_frame_index) {
            return frame.contains_entity(id);
        }
        false
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn set_current_frame(&mut self, current_frame_index: u32) {
        if let Some(frame_index) = self.frames.iter().position(|(_, range)| {
            range.0 <= current_frame_index || range.0 + range.1 > current_frame_index
        }) {
            self.current_frame_index = frame_index;
        } else {
            self.current_frame_index = std::usize::MAX;
        }
    }
}

#[derive(Debug, Clone)]
pub enum Frame {
    Key { entities: HashSet<Uuid> },
    Empty,
}

impl Frame {
    pub fn new() -> Self {
        Frame::Empty
    }

    pub fn contains_entity(&self, id: &Uuid) -> bool {
        match self {
            Frame::Key { entities } => entities.contains(id),
            Frame::Empty => false,
        }
    }

    pub fn add_entity(&mut self, id: &Uuid) {
        match self {
            Frame::Key { entities } => {
                entities.insert(*id);
            }
            Frame::Empty => {
                let mut entities = HashSet::new();
                entities.insert(*id);
                let mut new_frame = Frame::Key { entities };
                mem::swap(self, &mut new_frame);
            }
        }
    }

    pub fn remove_entity(&mut self, id: &Uuid) {
        if let Frame::Key { entities } = self {
            entities.remove(id);
            if entities.is_empty() {
                mem::swap(self, &mut Frame::Empty);
            }
        };
    }
}

impl Default for Frame {
    fn default() -> Self {
        Frame::Empty
    }
}
