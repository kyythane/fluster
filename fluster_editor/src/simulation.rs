#![deny(clippy::all)]
use crate::messages::{EditMessage, ToolMessage};
use crate::{
    rendering::RenderData,
    tools::{SelectionShape, ToolOption},
};
use fluster_core::rendering::{adjust_depth, PaintData};
use fluster_core::{
    runner::SceneData,
    types::{
        model::{DisplayLibraryItem, Entity, Part},
        shapes::{Edge, Shape},
    },
};
use pathfinder_color::ColorU;
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::{
    rect::RectF,
    vector::{Vector2F, Vector2I},
};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::mem;
use uuid::Uuid;

#[derive(Debug)]
pub struct SelectionHandle {
    entity_id: Uuid,
    part_id: Uuid,
    handles: Vec<VertexHandle>,
}

impl SelectionHandle {
    fn new(entity_id: Uuid, part_id: Uuid, handles: Vec<VertexHandle>) -> Self {
        Self {
            entity_id,
            part_id,
            handles,
        }
    }
}

// TODO: differentiate between control point and vertex?
#[derive(Debug)]
pub struct VertexHandle {
    position: Vector2F,
    edge_id: u32,
    library_id: Uuid,
}

impl VertexHandle {
    pub fn position(&self) -> &Vector2F {
        &self.position
    }
}

struct ShapeScratchPad {
    id: Uuid,
    edges: Vec<Edge>,
    committed_edges: usize,
    shape_prototype: Option<Shape>,
}

impl ShapeScratchPad {
    fn new() -> ShapeScratchPad {
        ShapeScratchPad {
            id: Uuid::new_v4(),
            edges: vec![],
            committed_edges: 0,
            shape_prototype: None,
        }
    }

    fn create_path_prototype(&mut self, options: &Vec<ToolOption>) {
        //TODO: Fill and StrokedFill, rename Path to Stroke
        let mut line_color = None;
        //let mut fill_color = None;
        let mut line_width = 1.0;
        let mut line_cap = LineCap::default();
        let mut line_join = LineJoin::default();
        let mut is_closed = false;
        for option in options {
            match option {
                ToolOption::LineColor(color) => line_color = *color,
                //   ToolOption::FillColor(color) => fill_color = *color,
                ToolOption::StrokeWidth(width) => line_width = *width,
                ToolOption::ClosedPath(closed) => is_closed = *closed,
                ToolOption::LineCap(cap) => line_cap = *cap,
                ToolOption::LineJoin(join) => line_join = *join,
                _ => {}
            }
        }
        self.shape_prototype = Some(Shape::Path {
            edges: vec![],
            color: line_color.unwrap_or(ColorU::black()),
            is_closed,
            stroke_style: StrokeStyle {
                line_width,
                line_cap,
                line_join,
            },
        });
    }

    fn update_library(&mut self, library: &mut HashMap<Uuid, DisplayLibraryItem>) {
        if let Some(shape_prototype) = &self.shape_prototype {
            let shape = match shape_prototype {
                Shape::Path {
                    color,
                    is_closed,
                    stroke_style,
                    ..
                } => Shape::Path {
                    edges: self.edges.clone(),
                    color: *color,
                    is_closed: *is_closed,
                    stroke_style: *stroke_style,
                },
                _ => todo!(),
            };

            library.insert(self.id, DisplayLibraryItem::Vector(shape));
        }
    }

    fn start_path(
        &mut self,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
        display_list: &mut HashMap<Uuid, Entity>,
        root_entity_id: &Uuid,
        start_position: Vector2F,
        options: &Vec<ToolOption>,
    ) {
        self.create_path_prototype(options);
        self.committed_edges = 1;
        self.id = Uuid::new_v4();
        self.edges.clear();
        self.edges.push(Edge::Move(start_position));
        let part = Part::new_vector(self.id, Transform2F::default(), None);
        display_list
            .entry(*root_entity_id)
            .and_modify(|root| root.add_part(part));
        self.update_library(library)
    }

    fn next_edge(
        &mut self,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
        next_position: Vector2F,
    ) {
        if self.committed_edges < self.edges.len() {
            self.edges.pop();
        }
        //TODO: other path types
        self.edges.push(Edge::Line(next_position));
        self.committed_edges = self.edges.len();
        self.update_library(library);
    }

    fn update_preview_edge(
        &mut self,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
        temp_position: Vector2F,
    ) {
        if self.committed_edges < self.edges.len() {
            self.edges.pop();
        }
        self.edges.push(Edge::Line(temp_position));
        self.update_library(library);
    }

    fn complete_path(
        &mut self,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
        display_list: &mut HashMap<Uuid, Entity>,
        root_entity_id: &Uuid,
    ) {
        if self.committed_edges < self.edges.len() {
            self.edges.pop();
        }
        if self.committed_edges <= 1 {
            library.remove(&self.id);
            display_list
                .entry(*root_entity_id)
                .and_modify(|root| root.remove_part(&self.id));
        } else {
            self.update_library(library);
        }
        self.edges.clear();
        self.committed_edges = 0;
        self.shape_prototype = None;
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
    scene_data: SceneData,
}

impl StageState {
    pub fn new(stage_size: Vector2I, background_color: ColorU) -> Self {
        let root_entity_id = Uuid::new_v4();
        let mut display_list = HashMap::new();
        display_list.insert(root_entity_id, Entity::create_root(root_entity_id));
        let mut new_self = Self {
            background_color,
            root_entity_id,
            library: HashMap::new(),
            display_list,
            size: stage_size,
            scale: 1.0,
            shape_scratch_pad: ShapeScratchPad::new(),
            scene_data: SceneData::new(stage_size.to_f32()),
        };
        // Need to init scene. Since StageState already knows how to set that up, just call into it
        new_self.update_scene();
        return new_self;
    }

    pub fn root(&self) -> &Uuid {
        &self.root_entity_id
    }

    pub fn apply_edit(&mut self, edit_message: &EditMessage) -> bool {
        match edit_message {
            EditMessage::ToolUpdate(tool_message) => {
                match tool_message {
                    ToolMessage::PathStart {
                        start_position,
                        options,
                    } => {
                        self.shape_scratch_pad.start_path(
                            &mut self.library,
                            &mut self.display_list,
                            &self.root_entity_id,
                            *start_position,
                            options,
                        );
                    }
                    ToolMessage::PathNext { next_position } => {
                        self.shape_scratch_pad
                            .next_edge(&mut self.library, *next_position);
                    }
                    ToolMessage::PathPlaceHover { hover_position } => {
                        self.shape_scratch_pad
                            .update_preview_edge(&mut self.library, *hover_position);
                    }
                    ToolMessage::PathEnd => {
                        self.shape_scratch_pad.complete_path(
                            &mut self.library,
                            &mut self.display_list,
                            &self.root_entity_id,
                        );
                        self.update_scene();
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn update_scene(&mut self) {
        self.scene_data
            .recompute(&self.root_entity_id, &mut self.display_list, &self.library)
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

    pub fn query_selection(&self, selection_shape: &SelectionShape) -> Vec<SelectionHandle> {
        match selection_shape {
            // Broadphase, collect all the parts with bounding boxes that overlap our query
            SelectionShape::None => vec![],
            SelectionShape::Point(point) => self.scene_data.quad_tree().query_point(point),
            SelectionShape::Area(rect) => self.scene_data.quad_tree().query_rect(rect),
        }
        .into_iter()
        .fold(
            HashMap::new(),
            |mut map: HashMap<Uuid, HashSet<Uuid>>, ((e_id, p_id), _)| {
                map.entry(e_id).or_default().insert(p_id);
                map
            },
        )
        .into_iter()
        .flat_map(|(e_id, p_ids)| {
            self.display_list
                .get(&e_id)
                .unwrap()
                .parts()
                .iter()
                .filter(move |part| p_ids.contains(part.item_id()))
                .map(move |part| {
                    let vertex_handles = self.collect_vertex_handles(selection_shape, part);
                    SelectionHandle::new(e_id, *part.item_id(), vertex_handles)
                })
        })
        .collect::<Vec<SelectionHandle>>()
    }

    // Narrow phase, Find all vertexes that overlap our query
    fn collect_vertex_handles(
        &self,
        selection_shape: &SelectionShape,
        part: &Part,
    ) -> Vec<VertexHandle> {
        match selection_shape {
            SelectionShape::None => vec![],
            SelectionShape::Point(point) => todo!(),
            SelectionShape::Area(rect) => todo!(),
        }
    }

    //TODO: how does root interact with layers? Should I support more than one root?
    pub fn compute_render_data(&self, timeline: &TimelineState) -> RenderData {
        let mut nodes = VecDeque::new();
        let mut depth_list = BTreeMap::new();
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
                }
                None => continue,
            }
        }

        RenderData::new(
            PaintData::new(depth_list),
            &self.scene_data,
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
