#![deny(clippy::all)]
use crate::{
    rendering::RenderData,
    tools::{EditMessage, ToolMessage},
};
use fluster_core::rendering::{adjust_depth, PaintData};
use fluster_core::types::{
    model::{DisplayLibraryItem, Entity, Part},
    shapes::{Edge, Shape},
};
use pathfinder_color::ColorU;
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::mem;
use uuid::Uuid;

struct ShapeScratchPad {
    id: Uuid,
    edges: Vec<Edge>,
    preview_edge: Option<Edge>,
}

impl ShapeScratchPad {
    fn new() -> ShapeScratchPad {
        ShapeScratchPad {
            id: Uuid::new_v4(),
            edges: vec![],
            preview_edge: None,
        }
    }

    fn init(&mut self, start_position: Vector2F) {
        self.preview_edge = None;
        self.id = Uuid::new_v4();
        self.edges.clear();
        self.edges.push(Edge::Move(start_position));
    }

    fn next_point(&mut self, next_position: Vector2F) {
        self.preview_edge = None;
        //TODO: other path types
        self.edges.push(Edge::Line(next_position));
    }

    fn update_preview_edge(&mut self, temp_position: Vector2F) {
        if self.edges.len() > 0 {
            self.preview_edge = Some(Edge::Line(temp_position));
        } else if self.preview_edge.is_some() {
            self.preview_edge = None;
        }
    }

    fn complete_shape(&mut self) -> (Uuid, Shape) {
        self.preview_edge = None;
        let edges = mem::take(&mut self.edges);
        (
            self.id,
            Shape::Path {
                points: edges,
                color: ColorU::black(),
                is_closed: false,
                stroke_style: StrokeStyle::default(),
            },
        )
    }
}
pub struct StageState {
    background_color: ColorU,
    root_entity_id: Uuid,
    library: HashMap<Uuid, DisplayLibraryItem>,
    display_list: HashMap<Uuid, Entity>,
    size: Vector2I,
    scale: f32,
    shape_scratch_pad: ShapeScratchPad,
}

impl StageState {
    pub fn new(stage_size: Vector2I, background_color: ColorU) -> Self {
        let root_entity_id = Uuid::new_v4();
        let mut display_list = HashMap::new();
        display_list.insert(root_entity_id, Entity::create_root(root_entity_id));
        Self {
            background_color,
            root_entity_id,
            library: HashMap::new(),
            display_list,
            size: stage_size,
            scale: 1.0,
            shape_scratch_pad: ShapeScratchPad::new(),
        }
    }

    pub fn root(&self) -> &Uuid {
        &self.root_entity_id
    }

    pub fn apply_edit(&mut self, edit_message: &EditMessage) -> bool {
        match edit_message {
            EditMessage::ToolUpdate(tool_message) => match tool_message {
                ToolMessage::PathStart { start_position } => {
                    self.shape_scratch_pad.init(*start_position);
                    false
                }
                ToolMessage::PathNext { next_position } => {
                    self.shape_scratch_pad.next_point(*next_position);
                    false
                }
                ToolMessage::PathPlaceHover { hover_position } => {
                    self.shape_scratch_pad.update_preview_edge(*hover_position);
                    false
                }
                ToolMessage::PathEnd => {
                    let (id, shape) = self.shape_scratch_pad.complete_shape();
                    self.library.insert(id, DisplayLibraryItem::Vector(shape));
                    let part = Part::Vector {
                        item_id: id,
                        transform: Transform2F::default(),
                        color: None,
                    };
                    self.display_list
                        .entry(self.root_entity_id)
                        .and_modify(|root| root.add_part(part));
                    true
                }
            },
            _ => false,
        }
    }

    #[inline]
    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn width(&self) -> i32 {
        self.size.x()
    }

    pub fn height(&self) -> i32 {
        self.size.y()
    }

    pub fn compute_render_data(&self, timeline: &TimelineState) -> RenderData {
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
                    } else if entity.id() == entity.parent() {
                        // If the parent id is the same as the entity id then we are at a root
                        world_space_transforms.insert(*entity.id(), *entity.transform());
                    } else {
                        continue;
                    }
                }
                None => continue,
            }
        }

        RenderData::new(
            PaintData::new(depth_list, world_space_transforms),
            self.background_color,
            &self.library,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct TimelineState {
    layers: Vec<LayerState>,
}

impl TimelineState {
    pub fn new(root_id: &Uuid) -> Self {
        let layer = LayerState::new(root_id);
        return Self {
            layers: vec![layer],
        };
    }

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

#[derive(Debug, Clone, Default)]
pub struct LayerState {
    frames: Vec<(FrameState, (u32, u32))>,
    current_frame_index: usize,
    visible: bool,
}

impl LayerState {
    pub fn new(root_id: &Uuid) -> Self {
        let frame_state = FrameState::from_entity(*root_id);
        Self {
            frames: vec![(frame_state, (0, 1))],
            current_frame_index: 0,
            visible: true,
        }
    }

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
pub enum FrameState {
    Key { entities: HashSet<Uuid> },
    Empty,
}

impl FrameState {
    pub fn from_entity(entity: Uuid) -> Self {
        let mut entities = HashSet::new();
        entities.insert(entity);
        Self::Key { entities }
    }

    pub fn contains_entity(&self, id: &Uuid) -> bool {
        match self {
            Self::Key { entities } => entities.contains(id),
            Self::Empty => false,
        }
    }

    pub fn add_entity(&mut self, id: &Uuid) {
        match self {
            Self::Key { entities } => {
                entities.insert(*id);
            }
            Self::Empty => {
                let mut entities = HashSet::new();
                entities.insert(*id);
                let new_frame = Self::Key { entities };
                mem::replace(self, new_frame);
            }
        }
    }

    pub fn remove_entity(&mut self, id: &Uuid) {
        if let Self::Key { entities } = self {
            entities.remove(id);
            if entities.is_empty() {
                mem::take(self);
            }
        };
    }
}

impl Default for FrameState {
    fn default() -> Self {
        Self::Empty
    }
}
