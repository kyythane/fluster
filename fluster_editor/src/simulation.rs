#![deny(clippy::all)]
use crate::messages::{EditMessage, SelectionHandle, VertexHandle};
use crate::{
    rendering::RenderData,
    scratch_pad::{ScratchPad, EDIT_LAYER},
    tools::SelectionShape,
};
use fluster_core::rendering::{adjust_depth, PaintData};
use fluster_core::{
    runner::{QuadTreeLayerOptions, SceneData},
    types::{
        model::{DisplayLibraryItem, Entity, Part},
        shapes::{Edge, Shape},
    },
};
use pathfinder_color::ColorU;
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::mem;
use uuid::Uuid;

pub struct StageState {
    background_color: ColorU,
    root_entity_id: Uuid,
    handle_container_id: Uuid,
    library: HashMap<Uuid, DisplayLibraryItem>,
    display_list: HashMap<Uuid, Entity>,
    size: Vector2I,
    scale: f32,
    scratch_pad: ScratchPad,
    scene_data: SceneData,
}

impl StageState {
    pub fn new(stage_size: Vector2I, background_color: ColorU) -> Self {
        let root_entity_id = Uuid::new_v4();
        let mut display_list = HashMap::new();
        display_list.insert(root_entity_id, Entity::create_root(root_entity_id));
        let handle_container_id = Uuid::new_v4();
        let mut new_self = Self {
            background_color,
            root_entity_id,
            handle_container_id,
            library: HashMap::new(),
            display_list,
            size: stage_size,
            scale: 1.0,
            scratch_pad: ScratchPad::default(),
            scene_data: SceneData::new(),
        };
        // NOTE: currently making edit collision 2x the stage size to allow for overdraw.
        new_self.scene_data.add_layer(
            EDIT_LAYER,
            RectF::new(stage_size.to_f32() * -1.0, stage_size.to_f32() * 2.0),
            QuadTreeLayerOptions::new(12.0),
        );
        // Need to init scene. Since StageState already knows how to set that up, just call into it
        new_self.update_scene();
        return new_self;
    }

    pub fn root(&self) -> &Uuid {
        &self.root_entity_id
    }

    pub fn draw_handles(&mut self, handles: Vec<SelectionHandle>) {
        let mut edges = vec![];
        for handle in handles {
            for vertex_handle in handle.vertex_handles() {
                edges.extend(
                    Edge::new_ellipse(
                        Vector2F::splat(5.0),
                        Transform2F::from_translation(*vertex_handle.position()),
                    )
                    .into_iter(),
                );
            }
        }
        println!("{:?}", edges);
        self.library.insert(
            self.handle_container_id,
            DisplayLibraryItem::Vector(Shape::Path {
                color: ColorU::new(192, 255, 0, 255),
                is_closed: false,
                edges,
                stroke_style: StrokeStyle {
                    line_width: 1.0,
                    line_cap: LineCap::default(),
                    line_join: LineJoin::default(),
                },
            }),
        );
    }

    pub fn apply_edit(&mut self, edit_message: &EditMessage) -> bool {
        // TODO: return a proper message type!
        match self.scratch_pad.apply_edit(
            edit_message,
            &mut self.library,
            &mut self.display_list,
            &mut self.scene_data,
            &self.root_entity_id,
        ) {
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
            SelectionShape::Point(point) => self
                .scene_data
                .quad_tree(&EDIT_LAYER)
                .unwrap()
                .0
                .query_point(point),
            SelectionShape::Area(rect) => self
                .scene_data
                .quad_tree(&EDIT_LAYER)
                .unwrap()
                .0
                .query_rect(rect),
        }
        .into_iter()
        .fold(
            // Since we query by part AABB, we need to collect them under the owning entity
            HashMap::new(),
            |mut map: HashMap<Uuid, HashSet<Uuid>>, ((e_id, p_id), _)| {
                map.entry(e_id).or_default().insert(p_id);
                map
            },
        )
        .into_iter()
        .flat_map(|(e_id, p_ids)| {
            let entity = self.display_list.get(&e_id).unwrap();
            let world_space_transform = self
                .scene_data
                .world_space_transforms()
                .get(entity.id())
                .unwrap();
            entity
                .parts()
                .filter(move |part| p_ids.contains(part.item_id()))
                .map(move |part| {
                    let vertex_handles = self.collect_vertex_handles(
                        selection_shape,
                        entity,
                        world_space_transform,
                        part,
                    );
                    SelectionHandle::new(e_id, *part.item_id(), vertex_handles)
                })
        })
        .collect::<Vec<SelectionHandle>>()
    }

    // Narrow phase, Find all vertexes that overlap our query
    fn collect_vertex_handles(
        &self,
        selection_shape: &SelectionShape,
        entity: &Entity,
        // Passing this in is kinda gross, but we don't want to refetch this multiple times per entity for a large selection
        world_space_transform: &Transform2F,
        part: &Part,
    ) -> Vec<VertexHandle> {
        let item_id = part.item_id();
        match self.library.get(item_id).unwrap() {
            DisplayLibraryItem::Vector(shape) => {
                let edges = shape
                    .edge_list(entity.morph_index())
                    .into_iter()
                    .enumerate();
                match selection_shape {
                    SelectionShape::None => vec![],
                    SelectionShape::Point(point) => edges
                        .flat_map(|(index, edge)| {
                            // TODO: configure handle radius (pixels)
                            edge.query_disk(point, 10.0, world_space_transform).map(
                                move |(vertex_index, distance, vertex)| {
                                    VertexHandle::new(
                                        *item_id,
                                        index,
                                        vertex_index,
                                        vertex,
                                        distance,
                                    )
                                },
                            )
                        })
                        .collect::<Vec<VertexHandle>>(),
                    SelectionShape::Area(rect) => edges
                        .flat_map(|(index, edge)| {
                            edge.query_rect(rect, world_space_transform).map(
                                move |(vertex_index, vertex)| {
                                    // Box selection has a separation of 0, since everything is inherently "inside" the specified area rather than near a point
                                    VertexHandle::new(*item_id, index, vertex_index, vertex, 0.0)
                                },
                            )
                        })
                        .collect::<Vec<VertexHandle>>(),
                }
            }
            DisplayLibraryItem::Raster(..) => todo!(),
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
