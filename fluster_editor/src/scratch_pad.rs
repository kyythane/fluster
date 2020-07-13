#![deny(clippy::all)]
use crate::messages::{EditMessage, Template, ToolMessage};
use crate::tools::{ToolOption, ToolOptionHandle};
use fluster_core::{
    actions::{ContainerCreationDefintition, ContainerCreationProperty},
    ecs::resources::{Library, QuadTreeLayer},
    engine::{Engine, SelectionHandle},
    types::shapes::{Edge, Shape},
};
use palette::LinSrgba;
use pathfinder_content::stroke::{LineCap, LineJoin, StrokeStyle};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use std::collections::HashMap;
use std::mem;
use uuid::Uuid;

pub const EDIT_LAYER: QuadTreeLayer = QuadTreeLayer::new(std::u32::MAX - 1);

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
        color: line_color.unwrap_or(LinSrgba::new(0.0, 0.0, 0.0, 1.0)),
        stroke_style: StrokeStyle {
            line_width,
            line_cap,
            line_join,
        },
    }
}

fn update_library(library: &mut Library, id: Uuid, shape_prototype: &Shape, edges: Vec<Edge>) {
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

    library.add_shape(id, shape);
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
        engine: &mut Engine,
    ) -> Result<bool, String> {
        // TODO: CLEANUP: Move bulk of ScratchPadState.apply_edit here
        self.state.apply_edit(edit_message, engine)
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
        engine: &mut Engine,
    ) -> Result<bool, String> {
        match edit_message {
            // TODO: this is gonna become very large. Maybe break it up into delegated functions
            EditMessage::ToolUpdate(tool_message) => match tool_message {
                ToolMessage::PathStart {
                    start_position,
                    options,
                } => {
                    if let Self::None = self {
                        let shape_scratch_pad =
                            ShapeScratchPad::start_path(*start_position, engine, options);
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
                        shape_scratch_pad.next_edge(&mut *engine.get_library_mut(), *next_position);
                        Ok(true)
                    } else {
                        Err("Unexpected Message \"PathNext\"".to_owned())
                    }
                }
                ToolMessage::PathPlaceHover { hover_position } => {
                    if let Self::NewPath(shape_scratch_pad) = self {
                        shape_scratch_pad
                            .update_preview_edge(&mut *engine.get_library_mut(), *hover_position);
                        Ok(true)
                    } else {
                        Err("Unexpected Message \"PathPlaceHover\"".to_owned())
                    }
                }
                ToolMessage::PathEnd => {
                    if let Self::NewPath(shape_scratch_pad) = self {
                        shape_scratch_pad.complete_path(engine);
                        mem::replace(self, Self::None);
                        Ok(true) //TODO: update scene message
                    } else {
                        Err("Unexpected Message \"PathEnd\"".to_owned())
                    }
                }
                ToolMessage::MovePointStart { selection_handle } => {
                    if let Self::None = self {
                        let mut vertex_scratch_pad =
                            VertexScratchPad::start_drag(engine, selection_handle)?;
                        mem::replace(self, Self::EditVertexes(vertex_scratch_pad));
                        Ok(false)
                    } else {
                        Err("Attempting to edit a vertex while edit in progress".to_owned())
                    }
                }
                ToolMessage::MovePointHover { hover_position } => {
                    if let Self::EditVertexes(vertex_scratch_pad) = self {
                        vertex_scratch_pad
                            .update_preview_drag(&mut *engine.get_library_mut(), *hover_position);
                        Ok(true)
                    } else {
                        Err("Unexpected Message \"MovePointHover\"".to_owned())
                    }
                }
                ToolMessage::MovePointEnd => {
                    if let Self::EditVertexes(vertex_scratch_pad) = self {
                        vertex_scratch_pad.complete_drag(engine);
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
                        let mut template_scratch_pad = TemplateShapeScratchpad::start(
                            engine,
                            *start_position,
                            options,
                            *template,
                        );
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
                        template_scratch_pad
                            .update_preview(&mut *engine.get_library_mut(), *hover_position)?;
                        Ok(true)
                    } else {
                        Err("Unexpected Message \"TemplatePlaceHover\"".to_owned())
                    }
                }
                ToolMessage::TemplateEnd => {
                    if let Self::NewTemplateShape(template_scratch_pad) = self {
                        template_scratch_pad.complete(engine)?;
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
    container_id: Uuid,
    item_id: Uuid,
    edges: Vec<Edge>,
    committed_edges: usize,
    shape_prototype: Shape,
    close_path: bool,
    selected_point: (usize, usize),
}

struct VertexScratchPad {
    container_id: Uuid,
    item_id: Uuid,
    edges: Vec<Edge>,
    shape_prototype: Shape,
    selected_point: (usize, usize),
}

struct TemplateShapeScratchpad {
    container_id: Uuid,
    item_id: Uuid,
    shape_prototype: Shape,
    start_position: Vector2F,
    end_position: Vector2F,
    template: Template,
    template_options: HashMap<ToolOptionHandle, ToolOption>,
}

impl ShapeScratchPad {
    fn start_path(
        start_position: Vector2F,
        engine: &mut Engine,
        options: &Vec<ToolOption>,
    ) -> Self {
        let mut close_path = false;
        options.iter().for_each(|option| match option {
            ToolOption::ClosedPath(close_path_opt) => close_path = *close_path_opt,
            _ => (),
        });
        let item_id = Uuid::new_v4();
        let container_id = Uuid::new_v4();
        let new_self = Self {
            container_id,
            item_id,
            edges: vec![Edge::Move(start_position)],
            committed_edges: 1,
            shape_prototype: create_shape_prototype(options),
            close_path,
            selected_point: (0, 0), // TODO: merge commited_edges and selected_point concept
        };
        new_self.update_library(&mut *engine.get_library_mut());
        engine.create_container(&ContainerCreationDefintition::new(
            *engine.root_container_id(),
            container_id,
            vec![
                ContainerCreationProperty::Display(item_id),
                ContainerCreationProperty::Layer(EDIT_LAYER),
            ],
        ));
        new_self
    }

    fn update_library(&self, library: &mut Library) {
        let mut edges = self.edges.clone();
        // add close after clone to keep end of list what is currently being edited
        if self.close_path {
            edges.push(Edge::Close);
        }
        update_library(library, self.item_id, &self.shape_prototype, edges);
    }

    fn next_edge(&mut self, library: &mut Library, next_position: Vector2F) {
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

    fn update_preview_edge(&mut self, library: &mut Library, temp_position: Vector2F) {
        if self.committed_edges < self.edges.len() {
            self.edges.pop();
        }
        self.edges.push(Edge::Line(temp_position));
        self.update_library(library);
    }

    fn complete_path(&mut self, engine: &mut Engine) {
        if self.committed_edges < self.edges.len() {
            self.edges.pop();
        }
        if self.committed_edges <= 1 {
            engine.get_library_mut().remove_shape(&self.item_id);
            engine.remove_container(&self.container_id);
        } else {
            if self.close_path {
                self.edges.push(Edge::Close);
            }
            update_library(
                &mut *engine.get_library_mut(),
                self.item_id,
                &self.shape_prototype,
                mem::take(&mut self.edges),
            );
            engine.mark_dirty(&self.container_id);
        }
    }
}

impl VertexScratchPad {
    fn start_drag(engine: &mut Engine, selection_handle: &SelectionHandle) -> Result<Self, String> {
        if let (Some(vertex), Some(item_id)) =
            (selection_handle.min_vertex(), selection_handle.shape_id())
        {
            if let Some(shape) = engine.get_library_mut().get_shape(item_id) {
                Ok(Self {
                    container_id: *selection_handle.container_id(),
                    item_id: *item_id,
                    edges: shape.edge_list(selection_handle.morph()),
                    shape_prototype: (*shape).clone(),
                    selected_point: (0, 0),
                })
            } else {
                Err(format!("Could not find library item {:?}", item_id))
            }
        } else {
            Err("Selection contained 0 vertexes".to_owned())
        }
    }

    fn update_preview_drag(&mut self, library: &mut Library, temp_position: Vector2F) {
        self.edges[self.selected_point.0].update_point(self.selected_point.1, temp_position);
        update_library(
            library,
            self.item_id,
            &self.shape_prototype,
            self.edges.clone(),
        );
    }

    fn complete_drag(&mut self, engine: &mut Engine) {
        update_library(
            &mut *engine.get_library_mut(),
            self.item_id,
            &self.shape_prototype,
            mem::take(&mut self.edges),
        );
        engine.mark_dirty(&self.container_id);
    }
}

impl TemplateShapeScratchpad {
    fn start(
        engine: &mut Engine,
        start_position: Vector2F,
        options: &Vec<ToolOption>,
        template: Template,
    ) -> Self {
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
        let new_self = Self {
            container_id: Uuid::new_v4(),
            item_id: Uuid::new_v4(),
            shape_prototype: create_shape_prototype(options),
            start_position,
            end_position: start_position,
            template,
            template_options,
        };
        update_library(
            &mut *engine.get_library_mut(),
            new_self.item_id,
            &new_self.shape_prototype,
            vec![],
        );
        engine.create_container(&ContainerCreationDefintition::new(
            *engine.root_container_id(),
            new_self.container_id,
            vec![
                ContainerCreationProperty::Display(new_self.item_id),
                ContainerCreationProperty::Layer(EDIT_LAYER),
            ],
        ));
        new_self
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

    fn update_preview(
        &mut self,
        library: &mut Library,
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

    fn complete(&mut self, engine: &mut Engine) -> Result<(), String> {
        if (self.end_position - self.start_position).length() < std::f32::EPSILON {
            engine.get_library_mut().remove_shape(&self.item_id);
            engine.remove_container(&self.container_id);
        } else {
            update_library(
                &mut *engine.get_library_mut(),
                self.item_id,
                &self.shape_prototype,
                self.compute_edge()?,
            );
            engine.mark_dirty(&self.container_id)
        }
        Ok(())
    }
}
