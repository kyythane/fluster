use crate::messages::EditMessage;
use crate::{
    scratch_pad::{ScratchPad, EDIT_LAYER},
    tools::SelectionShape,
};
use fluster_core::{
    ecs::resources::{FrameTime, Library, QuadTreeLayerOptions, QuadTreeQuery, QuadTrees},
    engine::{Engine, SelectionHandle},
    factories::new_display_container,
    types::{
        basic::{ContainerId, LibraryId},
        shapes::{Edge, Shape},
    },
};
use palette::{LinSrgb, LinSrgba};
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use std::collections::HashSet;
use std::{mem, time::Duration};

pub struct StageState<'a, 'b> {
    background_color: LinSrgb,
    root_container_id: ContainerId,
    handle_ids: (ContainerId, LibraryId),
    size: Vector2I,
    scale: f32,
    scratch_pad: ScratchPad,
    engine: Engine<'a, 'b>,
}

impl<'a, 'b> StageState<'a, 'b> {
    pub fn new(stage_size: Vector2I, background_color: LinSrgb) -> Self {
        let root_container_id = ContainerId::new();
        let mut quad_trees = QuadTrees::default();
        quad_trees.create_quad_tree(
            EDIT_LAYER,
            RectF::new(stage_size.to_f32() * -1.0, stage_size.to_f32() * 3.0),
            QuadTreeLayerOptions::new(12.0),
        );
        let mut engine = Engine::new(root_container_id, Library::default(), quad_trees);
        let handle_library_id = LibraryId::new();
        let handle_container_id = new_display_container(
            &mut engine,
            root_container_id,
            Transform2F::default(),
            handle_library_id,
        );
        let mut new_self = Self {
            background_color,
            root_container_id,
            handle_ids: (handle_container_id, handle_library_id),
            size: stage_size,
            scale: 1.0,
            scratch_pad: ScratchPad::default(),
            engine,
        };
        // Need to init the draw_handle container before we init scene data so we don't compute_bounds doesn't throw because it can't find a library item
        new_self.update_draw_handle(vec![]);
        // NOTE: currently making edit collision 3x the stage size to allow for overdraw.

        // Need to init scene. Since StageState already knows how to set that up, just call into it
        new_self.update_scene();
        return new_self;
    }

    pub fn background_color(&self) -> LinSrgb {
        self.background_color
    }

    pub fn engine(&self) -> &Engine<'a, 'b> {
        &self.engine
    }

    pub fn root(&self) -> &ContainerId {
        &self.root_container_id
    }

    pub fn draw_handles(&mut self, handles: Vec<SelectionHandle>) -> bool {
        let mut edges = vec![];
        for handle in handles {
            for vertex_handle in handle.handles() {
                edges.extend(
                    Edge::new_ellipse(
                        Vector2F::splat(5.0),
                        Transform2F::from_translation(vertex_handle.position()),
                    )
                    .into_iter(),
                );
            }
        }
        let redraw_needed = edges.len() > 0
            || self
                .engine
                .get_library()
                .get_shape(&self.handle_ids.1)
                .map(|shape| shape.edge_list(0.0).len() > 0)
                .unwrap_or_default();
        self.update_draw_handle(edges);
        redraw_needed
    }

    fn update_draw_handle(&mut self, edges: Vec<Edge>) {
        self.engine.get_library_mut().add_shape(
            self.handle_ids.1,
            Shape::Path {
                color: LinSrgba::new(0.3, 0.8, 0.7, 1.0),
                edges,
                stroke_style: StrokeStyle {
                    line_width: 2.0,
                    line_cap: LineCap::default(),
                    line_join: LineJoin::default(),
                },
            },
        );
    }

    pub fn apply_edit(&mut self, edit_message: &EditMessage) -> bool {
        // TODO: return a proper message type!
        match self.scratch_pad.apply_edit(edit_message, &mut self.engine) {
            Ok(res) => {
                if res {
                    self.update_scene();
                    true
                } else {
                    false
                }
            }
            Err(error) => {
                println!("{:}", error);
                false
            }
        }
    }

    pub fn update_scene(&mut self) {
        self.engine.update(FrameTime {
            delta_frame: 1,
            delta_time: Duration::from_secs_f64(1.0 / 60.0),
        })
    }

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
            SelectionShape::None => vec![],
            SelectionShape::Point(point) => self
                .engine
                .spatial_query(&QuadTreeQuery::Point(EDIT_LAYER, *point)),
            SelectionShape::Area(rect) => self
                .engine
                .spatial_query(&QuadTreeQuery::Rect(EDIT_LAYER, *rect)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TimelineState {
    layers: Vec<LayerState>,
}

impl TimelineState {
    pub fn new(root_id: &ContainerId) -> Self {
        let layer = LayerState::new(root_id);
        return Self {
            layers: vec![layer],
        };
    }

    pub fn can_show_entity(&self, id: &ContainerId) -> bool {
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
    pub fn new(root_id: &ContainerId) -> Self {
        let frame_state = FrameState::from_entity(*root_id);
        Self {
            frames: vec![(frame_state, (0, 1))],
            current_frame_index: 0,
            visible: true,
        }
    }

    pub fn can_show_entity(&self, id: &ContainerId) -> bool {
        if !self.visible {
            return false;
        }
        self.contains_entity(id)
    }

    pub fn contains_entity(&self, id: &ContainerId) -> bool {
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
    Key { entities: HashSet<ContainerId> },
    Empty,
}

impl FrameState {
    pub fn from_entity(entity: ContainerId) -> Self {
        let mut entities = HashSet::new();
        entities.insert(entity);
        Self::Key { entities }
    }

    pub fn contains_entity(&self, id: &ContainerId) -> bool {
        match self {
            Self::Key { entities } => entities.contains(id),
            Self::Empty => false,
        }
    }

    pub fn add_entity(&mut self, id: &ContainerId) {
        match self {
            Self::Key { entities } => {
                entities.insert(*id);
            }
            Self::Empty => {
                let mut entities = HashSet::new();
                entities.insert(*id);
                let new_frame = Self::Key { entities };
                *self = new_frame;
            }
        }
    }

    pub fn remove_entity(&mut self, id: &ContainerId) {
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
