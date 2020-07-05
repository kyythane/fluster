#![deny(clippy::all)]
use crate::messages::{EditMessage, SelectionHandle, Template, ToolMessage};
use crate::tools::{ToolOption, ToolOptionHandle};
use fluster_core::{
    runner::{QuadTreeLayer, SceneData},
    types::{
        model::{DisplayLibraryItem, Entity, Part},
        shapes::{Edge, Shape},
    },
};
use pathfinder_color::ColorU;
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use std::collections::HashMap;
use std::mem;
use uuid::Uuid;

pub const EDIT_LAYER: QuadTreeLayer = std::u32::MAX - 1;

fn create_shape_prototype(options: &Vec<ToolOption>) -> Shape {
    //TODO: Fill and StrokedFill, rename Path to Stroke
    let mut line_color = None;
    //let mut fill_color = None;
    let mut line_width = 1.0;
    let mut line_cap = LineCap::default();
    let mut line_join = LineJoin::default();
    for option in options {
        match option {
            ToolOption::LineColor(color) => line_color = *color,
            //   ToolOption::FillColor(color) => fill_color = *color,
            ToolOption::StrokeWidth(width) => line_width = *width,
            ToolOption::LineCap(cap) => line_cap = *cap,
            ToolOption::LineJoin(join) => line_join = *join,
            _ => {}
        }
    }
    Shape::Path {
        edges: vec![],
        color: line_color.unwrap_or(ColorU::black()),
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
            stroke_style,
            ..
        } => Shape::Path {
            edges,
            color: *color,
            stroke_style: *stroke_style,
        },
        _ => todo!(),
    };

    library.insert(id, DisplayLibraryItem::Vector(shape));
}

pub struct ScratchPad {
    state: ScratchPadState,
}

impl Default for ScratchPad {
    fn default() -> Self {
        Self {
            state: ScratchPadState::default(),
        }
    }
}

impl ScratchPad {
    pub fn apply_edit(
        &mut self,
        edit_message: &EditMessage,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
        display_list: &mut HashMap<Uuid, Entity>,
        scene_data: &mut SceneData,
        root_entity_id: &Uuid,
    ) -> Result<bool, String> {
        // TODO: CLEANUP: Move bulk of ScratchPadState.apply_edit here
        self.state.apply_edit(
            edit_message,
            library,
            display_list,
            scene_data,
            root_entity_id,
        )
    }
}

enum ScratchPadState {
    NewPath(ShapeScratchPad),
    NewTemplateShape(TemplateShapeScratchpad),
    EditVertexes(VertexScratchPad),
    None,
}

impl Default for ScratchPadState {
    fn default() -> Self {
        Self::None
    }
}

impl ScratchPadState {
    fn apply_edit(
        &mut self,
        edit_message: &EditMessage,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
        display_list: &mut HashMap<Uuid, Entity>,
        scene_data: &mut SceneData,
        root_entity_id: &Uuid,
    ) -> Result<bool, String> {
        match edit_message {
            // TODO: this is gonna become very large. Maybe break it up into delegated functions
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
                        shape_scratch_pad.complete_path(
                            library,
                            display_list,
                            scene_data,
                            root_entity_id,
                        );
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
                        vertex_scratch_pad.complete_drag(display_list, library);
                        mem::replace(self, Self::None);
                        Ok(true) //TODO: update scene message
                    } else {
                        Err("Unexpected Message \"PathEnd\"".to_owned())
                    }
                }
                ToolMessage::TemplateStart {
                    start_position,
                    options,
                    template,
                } => {
                    if let Self::None = self {
                        let mut template_scratch_pad =
                            TemplateShapeScratchpad::init(*start_position, options, *template);
                        template_scratch_pad.start(library, display_list, root_entity_id);
                        mem::replace(self, Self::NewTemplateShape(template_scratch_pad));
                        Ok(false)
                    } else {
                        Err(
                            "Attempting to start a new ellipse while an edit was in progress"
                                .to_owned(),
                        )
                    }
                }
                ToolMessage::TemplatePlaceHover { hover_position } => {
                    if let Self::NewTemplateShape(template_scratch_pad) = self {
                        template_scratch_pad.update_preview(library, *hover_position)?;
                        Ok(true)
                    } else {
                        Err("Unexpected Message \"TemplatePlaceHover\"".to_owned())
                    }
                }
                ToolMessage::TemplateEnd => {
                    if let Self::NewTemplateShape(template_scratch_pad) = self {
                        template_scratch_pad.complete(
                            library,
                            display_list,
                            scene_data,
                            root_entity_id,
                        )?;
                        mem::replace(self, Self::None);
                        Ok(true) //TODO: update scene message
                    } else {
                        Err("Unexpected Message \"TemplateEnd\"".to_owned())
                    }
                }
            },
            // This isn't the kind of message that scratchpad handles, so just move on
            _ => Ok(false),
        }
    }
}
struct ShapeScratchPad {
    part_id: Uuid,
    item_id: Uuid,
    edges: Vec<Edge>,
    committed_edges: usize,
    shape_prototype: Shape,
    close_path: bool,
    selected_point: (usize, usize),
}

struct VertexScratchPad {
    item_id: Uuid,
    entity_id: Uuid,
    edges: Vec<Edge>,
    shape_prototype: Shape,
    selected_point: (usize, usize),
}

struct TemplateShapeScratchpad {
    part_id: Uuid,
    item_id: Uuid,
    shape_prototype: Shape,
    start_position: Vector2F,
    end_position: Vector2F,
    template: Template,
    template_options: HashMap<ToolOptionHandle, ToolOption>,
}

impl ShapeScratchPad {
    fn init(options: &Vec<ToolOption>) -> Self {
        let mut close_path = false;
        options.iter().for_each(|option| match option {
            ToolOption::ClosedPath(close_path_opt) => close_path = *close_path_opt,
            _ => (),
        });
        Self {
            part_id: Uuid::new_v4(),
            item_id: Uuid::new_v4(),
            edges: vec![],
            committed_edges: 0,
            shape_prototype: create_shape_prototype(options),
            close_path,
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
        let mut part = Part::new_vector(self.item_id, Transform2F::default(), None);
        part.add_to_collision_layer(EDIT_LAYER);
        display_list.entry(*root_entity_id).and_modify(|root| {
            root.add_part(&self.part_id, part);
            // Mark the root clean since we don't want to recompute bounds until the shape is done
            root.mark_clean();
        });
        self.update_library(library);
    }

    fn update_library(&self, library: &mut HashMap<Uuid, DisplayLibraryItem>) {
        let mut edges = self.edges.clone();
        // add close after clone to keep end of list what is currently being edited
        if self.close_path {
            edges.push(Edge::Close);
        }
        update_library(library, self.item_id, &self.shape_prototype, edges);
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
        let mut edges = self.edges.clone();
        if self.close_path {
            edges.push(Edge::Close);
        }
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
        scene_data: &mut SceneData,
        root_entity_id: &Uuid,
    ) {
        if self.committed_edges < self.edges.len() {
            self.edges.pop();
        }
        if self.committed_edges <= 1 {
            library.remove(&self.item_id);
            display_list.entry(*root_entity_id).and_modify(|root| {
                root.remove_part(&self.part_id);
                root.mark_dirty();
            });
            scene_data
                .quad_tree_mut(&EDIT_LAYER)
                .unwrap()
                .0
                .remove(&(*root_entity_id, self.part_id));
        } else {
            if self.close_path {
                self.edges.push(Edge::Close);
            }
            update_library(
                library,
                self.item_id,
                &self.shape_prototype,
                mem::take(&mut self.edges),
            );
            display_list.entry(*root_entity_id).and_modify(|root| {
                root.mark_dirty();
            });
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
                    entity_id: *selection_handle.entity_id(),
                    item_id: Uuid::new_v4(),
                    edges: vec![],
                    shape_prototype: shape.clone(),
                    selected_point: (0, 0),
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
                self.item_id = *vertex.library_id();
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
        update_library(
            library,
            self.item_id,
            &self.shape_prototype,
            self.edges.clone(),
        );
    }

    fn complete_drag(
        &mut self,
        display_list: &mut HashMap<Uuid, Entity>,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
    ) {
        update_library(
            library,
            self.item_id,
            &self.shape_prototype,
            mem::take(&mut self.edges),
        );
        display_list.entry(self.entity_id).and_modify(|root| {
            root.mark_dirty();
        });
    }
}

impl TemplateShapeScratchpad {
    fn init(start_position: Vector2F, options: &Vec<ToolOption>, template: Template) -> Self {
        let mut edges = 5;
        let mut corner_radius: f32 = 0.0;
        let mut use_super_ellipse_approximation = false;
        options.iter().for_each(|option| match option {
            ToolOption::NumEdges(option_edges) => edges = *option_edges,
            ToolOption::CornerRadius(option_radius) => corner_radius = *option_radius,
            ToolOption::UseSuperEllipseApproximation(use_approximation) => {
                use_super_ellipse_approximation = *use_approximation
            }
            _ => (),
        });
        let mut template_options = HashMap::new();
        template_options.insert(ToolOptionHandle::NumEdges, ToolOption::NumEdges(edges));
        template_options.insert(
            ToolOptionHandle::CornerRadius,
            ToolOption::CornerRadius(corner_radius),
        );
        template_options.insert(
            ToolOptionHandle::UseSuperEllipseApproximation,
            ToolOption::UseSuperEllipseApproximation(use_super_ellipse_approximation),
        );
        Self {
            part_id: Uuid::new_v4(),
            item_id: Uuid::new_v4(),
            shape_prototype: create_shape_prototype(options),
            start_position,
            end_position: start_position,
            template,
            template_options,
        }
    }

    fn compute_edge(&self) -> Result<Vec<Edge>, String> {
        let corner_radius = if let Some(ToolOption::CornerRadius(corner_radius)) =
            self.template_options.get(&ToolOptionHandle::CornerRadius)
        {
            *corner_radius
        } else {
            return Err(
                "Attempting to draw a polygon without specifying the corner radius".to_owned(),
            );
        };
        match self.template {
            Template::Ellipse => Ok(Edge::new_ellipse(
                self.end_position - self.start_position,
                Transform2F::from_translation(self.start_position),
            )),
            Template::Rectangle => {
                let size = Vector2F::new(
                    (self.end_position.x() - self.start_position.x()).abs(),
                    (self.end_position.y() - self.start_position.y()).abs(),
                );
                if corner_radius.abs() < std::f32::EPSILON {
                    Ok(Edge::new_rect(
                        size,
                        Transform2F::from_translation(self.start_position.min(self.end_position)),
                    ))
                } else {
                    if let Some(ToolOption::UseSuperEllipseApproximation(true)) = self
                        .template_options
                        .get(&ToolOptionHandle::UseSuperEllipseApproximation)
                    {
                        Ok(Edge::new_superellipse(
                            size,
                            corner_radius,
                            Transform2F::from_translation(
                                self.start_position.min(self.end_position),
                            ),
                        ))
                    } else {
                        Ok(Edge::new_round_rect(
                            size,
                            corner_radius,
                            Transform2F::from_translation(
                                self.start_position.min(self.end_position),
                            ),
                        ))
                    }
                }
            }
            Template::Polygon => {
                let edges = if let Some(ToolOption::NumEdges(edges)) =
                    self.template_options.get(&ToolOptionHandle::NumEdges)
                {
                    *edges
                } else {
                    return Err(
                        "Attempting to draw a polygon without specifying number of sides"
                            .to_owned(),
                    );
                };
                let angle = if (self.end_position.x() - self.start_position.x()).abs()
                    > std::f32::EPSILON
                {
                    (self.end_position.y() - self.start_position.y())
                        .atan2(self.end_position.x() - self.start_position.x())
                } else {
                    0.0
                };
                if corner_radius.abs() < std::f32::EPSILON {
                    Ok(Edge::new_polygon(
                        edges,
                        2.0 * (self.end_position - self.start_position).length()
                            * (std::f32::consts::PI / (edges as f32)).sin(),
                        Transform2F::from_translation(self.start_position)
                            * Transform2F::from_rotation(angle),
                    ))
                } else {
                    Ok(Edge::new_round_polygon(
                        edges,
                        2.0 * (self.end_position - self.start_position).length()
                            * (std::f32::consts::PI / (edges as f32)).sin(),
                        corner_radius,
                        Transform2F::from_translation(self.start_position)
                            * Transform2F::from_rotation(angle),
                    ))
                }
            }
        }
    }

    fn start(
        &mut self,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
        display_list: &mut HashMap<Uuid, Entity>,
        root_entity_id: &Uuid,
    ) {
        let mut part = Part::new_vector(self.item_id, Transform2F::default(), None);
        part.add_to_collision_layer(EDIT_LAYER);
        display_list.entry(*root_entity_id).and_modify(|root| {
            root.add_part(&self.part_id, part);
            // Mark the root clean since we don't want to recompute bounds until the shape is done
            root.mark_clean();
        });
        update_library(library, self.item_id, &self.shape_prototype, vec![]);
    }

    fn update_preview(
        &mut self,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
        temp_position: Vector2F,
    ) -> Result<(), String> {
        self.end_position = temp_position;
        update_library(
            library,
            self.item_id,
            &self.shape_prototype,
            self.compute_edge()?,
        );
        Ok(())
    }

    fn complete(
        &mut self,
        library: &mut HashMap<Uuid, DisplayLibraryItem>,
        display_list: &mut HashMap<Uuid, Entity>,
        scene_data: &mut SceneData,
        root_entity_id: &Uuid,
    ) -> Result<(), String> {
        if (self.end_position - self.start_position).length() < std::f32::EPSILON {
            library.remove(&self.item_id);
            display_list.entry(*root_entity_id).and_modify(|root| {
                root.remove_part(&self.part_id);
            });
            scene_data
                .quad_tree_mut(&EDIT_LAYER)
                .unwrap()
                .0
                .remove(&(*root_entity_id, self.part_id));
        } else {
            update_library(
                library,
                self.item_id,
                &self.shape_prototype,
                self.compute_edge()?,
            );
            display_list.entry(*root_entity_id).and_modify(|root| {
                root.mark_dirty();
            });
        }
        Ok(())
    }
}
