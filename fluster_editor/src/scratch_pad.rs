#![deny(clippy::all)]
use crate::messages::{EditMessage, SelectionHandle, ToolMessage};
use crate::tools::ToolOption;
use fluster_core::types::{
    model::{DisplayLibraryItem, Entity, Part},
    shapes::{Edge, Shape},
};
use pathfinder_color::ColorU;
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use std::collections::HashMap;
use std::mem;
use uuid::Uuid;

fn create_shape_prototype(options: &Vec<ToolOption>) -> Shape {
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
    Shape::Path {
        edges: vec![],
        color: line_color.unwrap_or(ColorU::black()),
        is_closed,
        stroke_style: StrokeStyle {
            line_width,
            line_cap,
            line_join,
        },
    }
}

fn update_library(
    library: &mut HashMap<Uuid, DisplayLibraryItem>,
    id: Uuid,
    shape_prototype: &Shape,
    edges: Vec<Edge>,
) {
    let shape = match shape_prototype {
        Shape::Path {
            color,
            is_closed,
            stroke_style,
            ..
        } => Shape::Path {
            edges,
            color: *color,
            is_closed: *is_closed,
            stroke_style: *stroke_style,
        },
        _ => todo!(),
    };

    library.insert(id, DisplayLibraryItem::Vector(shape));
}

pub enum ScratchPad {
    NewPath(ShapeScratchPad),
    NewPolygon(),
    NewElipse(),
    EditVertexes(VertexScratchPad),
    MoveShapes(TransformScratchPad),
    None,
}

impl Default for ScratchPad {
    fn default() -> Self {
        Self::None
    }
}

impl ScratchPad {
    pub fn apply_edit(
        &mut self,
        edit_message: &EditMessage,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
        display_list: &mut HashMap<Uuid, Entity>,
        root_entity_id: &Uuid,
    ) -> Result<bool, String> {
        match edit_message {
            EditMessage::ToolUpdate(tool_message) => match tool_message {
                ToolMessage::PathStart {
                    start_position,
                    options,
                } => {
                    if let Self::None = self {
                        let mut shape_scratch_pad = ShapeScratchPad::init(options);
                        shape_scratch_pad.start_path(
                            library,
                            display_list,
                            root_entity_id,
                            *start_position,
                        );
                        mem::replace(self, Self::NewPath(shape_scratch_pad));
                        Ok(false)
                    } else {
                        Err(
                            "Attempting to start a new path while an edit was in progress"
                                .to_owned(),
                        )
                    }
                }
                ToolMessage::PathNext { next_position } => {
                    if let Self::NewPath(shape_scratch_pad) = self {
                        shape_scratch_pad.next_edge(library, *next_position);
                        Ok(true)
                    } else {
                        Err("Unexpected Message \"PathNext\"".to_owned())
                    }
                }
                ToolMessage::PathPlaceHover { hover_position } => {
                    if let Self::NewPath(shape_scratch_pad) = self {
                        shape_scratch_pad.update_preview_edge(library, *hover_position);
                        Ok(true)
                    } else {
                        Err("Unexpected Message \"PathPlaceHover\"".to_owned())
                    }
                }
                ToolMessage::PathEnd => {
                    if let Self::NewPath(shape_scratch_pad) = self {
                        shape_scratch_pad.complete_path(library, display_list, root_entity_id);
                        mem::replace(self, Self::None);
                        Ok(true) //TODO: update scene message
                    } else {
                        Err("Unexpected Message \"PathEnd\"".to_owned())
                    }
                }
                ToolMessage::MovePointStart { selection_handle } => {
                    if let Self::None = self {
                        let mut vertex_scratch_pad =
                            VertexScratchPad::init(selection_handle, library)?;
                        vertex_scratch_pad.start_drag(selection_handle, display_list, library);
                        mem::replace(self, Self::EditVertexes(vertex_scratch_pad));
                        Ok(false)
                    } else {
                        Err("Attempting to edit a vertex while edit in progress".to_owned())
                    }
                }
                ToolMessage::MovePointHover { hover_position } => {
                    if let Self::EditVertexes(vertex_scratch_pad) = self {
                        vertex_scratch_pad.update_preview_drag(library, *hover_position);
                        Ok(true)
                    } else {
                        Err("Unexpected Message \"MovePointHover\"".to_owned())
                    }
                }
                ToolMessage::MovePointEnd => {
                    if let Self::EditVertexes(vertex_scratch_pad) = self {
                        vertex_scratch_pad.complete_drag(library);
                        mem::replace(self, Self::None);
                        Ok(true) //TODO: update scene message
                    } else {
                        Err("Unexpected Message \"PathEnd\"".to_owned())
                    }
                }
            },
            // This isn't the kind of message that scratchpad handles, so just move on
            _ => Ok(false),
        }
    }
}

pub struct ShapeScratchPad {
    id: Uuid,
    edges: Vec<Edge>,
    committed_edges: usize,
    shape_prototype: Shape,
    selected_point: (usize, usize),
}

pub struct VertexScratchPad {
    id: Uuid,
    edges: Vec<Edge>,
    shape_prototype: Shape,
    selected_point: (usize, usize),
}

pub struct TransformScratchPad {}

impl ShapeScratchPad {
    fn init(options: &Vec<ToolOption>) -> Self {
        Self {
            id: Uuid::new_v4(),
            edges: vec![],
            committed_edges: 0,
            shape_prototype: create_shape_prototype(options),
            selected_point: (0, 0), // TODO: merge commited_edges and selected_point concept
        }
    }

    fn start_path(
        &mut self,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
        display_list: &mut HashMap<Uuid, Entity>,
        root_entity_id: &Uuid,
        start_position: Vector2F,
    ) {
        self.committed_edges = 1;
        self.edges.push(Edge::Move(start_position));
        let part = Part::new_vector(self.id, Transform2F::default(), None);
        display_list
            .entry(*root_entity_id)
            .and_modify(|root| root.add_part(part));
        update_library(library, self.id, &self.shape_prototype, self.edges.clone());
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
        update_library(library, self.id, &self.shape_prototype, self.edges.clone());
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
        update_library(library, self.id, &self.shape_prototype, self.edges.clone());
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
            update_library(
                library,
                self.id,
                &self.shape_prototype,
                mem::take(&mut self.edges),
            );
        }
    }
}

impl VertexScratchPad {
    fn init(
        selection_handle: &SelectionHandle,
        library: &HashMap<Uuid, DisplayLibraryItem>,
    ) -> Result<Self, String> {
        if let Some(vertex) = selection_handle.min_vertex() {
            if let Some(DisplayLibraryItem::Vector(shape)) = library.get(vertex.library_id()) {
                Ok(Self {
                    id: Uuid::new_v4(),
                    edges: vec![],
                    shape_prototype: shape.clone(),
                    selected_point: (0, 0), // TODO: merge commited_edges and selected_point concept
                })
            } else {
                Err(format!(
                    "Could not find library item {:?}",
                    vertex.library_id()
                ))
            }
        } else {
            Err("Selection contained 0 vertexes".to_owned())
        }
    }

    fn start_drag(
        &mut self,
        selection_handle: &SelectionHandle,
        display_list: &HashMap<Uuid, Entity>,
        library: &HashMap<Uuid, DisplayLibraryItem>,
    ) {
        if let Some(vertex) = selection_handle.min_vertex() {
            if let Some(DisplayLibraryItem::Vector(shape)) = library.get(vertex.library_id()) {
                let morph_index =
                    if let Some(entity) = display_list.get(&selection_handle.entity_id()) {
                        entity.morph_index()
                    } else {
                        0.0
                    };
                self.id = *vertex.library_id();
                self.edges = shape.edge_list(morph_index);
                self.selected_point = (vertex.edge_id(), vertex.vertex_id());
            }
        }
    }

    fn update_preview_drag(
        &mut self,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
        temp_position: Vector2F,
    ) {
        self.edges[self.selected_point.0].update_point(self.selected_point.1, temp_position);
        update_library(library, self.id, &self.shape_prototype, self.edges.clone());
    }

    fn complete_drag(&mut self, library: &mut HashMap<Uuid, DisplayLibraryItem>) {
        update_library(
            library,
            self.id,
            &self.shape_prototype,
            mem::take(&mut self.edges),
        );
    }
}
