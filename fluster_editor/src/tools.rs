#![deny(clippy::all)]
use crate::messages::{AppMessage, EditMessage, ToolMessage};
use iced::{Checkbox, Column, Radio, Row, Text, TextInput};
use iced_native::{
    image::Handle as ImageHandle, input::mouse::Button as MouseButton,
    input::mouse::Event as MouseEvent, input::ButtonState, text_input::State as TextInputState,
    MouseCursor,
};
use pathfinder_color::ColorU;
use pathfinder_content::stroke::{LineCap, LineJoin};
use pathfinder_geometry::{rect::RectF, vector::Vector2F};
use std::collections::{HashMap, HashSet};
use std::mem;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub enum Tool {
    Pointer,
    Path,
    Polygon,
    Ellipse,
    Fill,
    Eyedropper,
}

impl Tool {
    pub fn image_handle(&self) -> ImageHandle {
        ImageHandle::from_path(format!(
            "{}/{}",
            env!("CARGO_MANIFEST_DIR"),
            match self {
                Self::Pointer => "assets/icons/030-mouse.png",
                Self::Path => "assets/icons/033-pen tool.png",
                _ => "assets/icons/020-graphic tool.png",
            }
        ))
    }

    pub fn change_message(&self) -> EditMessage {
        EditMessage::ToolChange(*self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectionShape {
    None,
    Point(Vector2F),
    Area(RectF),
}

#[derive(Clone, Copy, Debug)]
enum PlacementState {
    None,
    Placing,
}

#[derive(Clone, Debug)]
enum ToolState {
    Pointer, //TODO: grab: edge, point, fill, scale_x, scale_y, scale_xy, entity, group. hover
    Path {
        placement_state: PlacementState,
    },
    Polygon {
        center: Vector2F,
        lock_aspect_ratio: bool,
        placement_state: PlacementState,
    },
    Ellipse {
        lock_aspect_ratio: bool,
        focus_1: Vector2F,
        focus_2: Vector2F,
        placement_state: PlacementState,
    },
    Fill {
        color: ColorU,
    },
    Eyedropper {
        color: ColorU,
    },
}

impl ToolState {
    fn new(tool: Tool) -> Self {
        match tool {
            Tool::Pointer => Self::Pointer,
            Tool::Path => Self::Path {
                placement_state: PlacementState::None,
            },
            Tool::Polygon => Self::Polygon {
                center: Vector2F::default(),
                lock_aspect_ratio: false,
                placement_state: PlacementState::None,
            },
            Tool::Ellipse => Self::Ellipse {
                focus_1: Vector2F::default(),
                focus_2: Vector2F::default(),
                lock_aspect_ratio: false,
                placement_state: PlacementState::None,
            },
            _ => todo!(),
        }
    }

    fn switch_tool(&mut self, tool: Tool) {
        mem::replace(self, Self::new(tool));
    }

    fn cancel_action(&mut self) {
        mem::replace(self, Self::new(self.tool()));
    }

    fn tool(&self) -> Tool {
        match self {
            Self::Pointer => Tool::Pointer,
            Self::Path { .. } => Tool::Path,
            Self::Polygon { .. } => Tool::Polygon,
            Self::Ellipse { .. } => Tool::Ellipse,
            Self::Fill { .. } => Tool::Fill,
            Self::Eyedropper { .. } => Tool::Eyedropper,
        }
    }

    fn placement_state(&self) -> PlacementState {
        match self {
            Self::Pointer => PlacementState::None,
            Self::Fill { .. } => PlacementState::None,
            Self::Eyedropper { .. } => PlacementState::None,
            Self::Path {
                placement_state, ..
            }
            | Self::Polygon {
                placement_state, ..
            }
            | Self::Ellipse {
                placement_state, ..
            } => *placement_state,
        }
    }

    fn start_placing(&mut self) {
        match self {
            Self::Path { .. } => {
                mem::replace(
                    self,
                    Self::Path {
                        placement_state: PlacementState::Placing,
                    },
                );
            }
            _ => (),
        }
    }

    fn stop_placing(&mut self) {
        match self {
            Self::Path { .. } => {
                mem::replace(
                    self,
                    Self::Path {
                        placement_state: PlacementState::None,
                    },
                );
            }
            _ => (),
        }
    }

    fn mouse_cursor(&self) -> MouseCursor {
        match self {
            Self::Pointer => MouseCursor::Idle,
            _ => match self.placement_state() {
                PlacementState::None => MouseCursor::Pointer,
                PlacementState::Placing => MouseCursor::Grabbing,
            },
        }
    }

    fn selection_shape(&self, stage_position: Vector2F) -> SelectionShape {
        SelectionShape::Point(stage_position)
    }

    fn on_mouse_event(
        &self,
        mouse_event: MouseEvent,
        stage_position: Vector2F,
        tool_options: Vec<ToolOption>,
    ) -> Option<ToolMessage> {
        let tool_options = self.get_options(tool_options);
        match self {
            Self::Pointer => None,
            Self::Path {
                placement_state, ..
            } => match mouse_event {
                MouseEvent::Input { state, button }
                    if state == ButtonState::Pressed && button == MouseButton::Left =>
                {
                    match placement_state {
                        PlacementState::None => Some(ToolMessage::PathStart {
                            start_position: stage_position,
                            options: tool_options,
                        }),
                        PlacementState::Placing => Some(ToolMessage::PathNext {
                            next_position: stage_position,
                        }),
                    }
                }
                MouseEvent::Input { state, .. } if state == ButtonState::Pressed => {
                    match placement_state {
                        PlacementState::None => None,
                        PlacementState::Placing => Some(ToolMessage::PathEnd),
                    }
                }
                MouseEvent::CursorMoved { .. } => {
                    if let PlacementState::Placing = placement_state {
                        Some(ToolMessage::PathPlaceHover {
                            hover_position: stage_position,
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => todo!(),
        }
    }

    fn update(&mut self, tool_message: &ToolMessage) {
        match tool_message {
            ToolMessage::PathStart { .. } => self.start_placing(),
            ToolMessage::PathEnd { .. } => self.stop_placing(),
            _ => (),
        };
    }

    fn get_options(&self, mut tool_options: Vec<ToolOption>) -> Vec<ToolOption> {
        tool_options
            .drain(..)
            .filter(|option| match option {
                ToolOption::LineColor(..) => match self {
                    Self::Path { .. } | Self::Ellipse { .. } | Self::Polygon { .. } => true,
                    _ => false,
                },
                ToolOption::StrokeWidth(..) => match self {
                    Self::Path { .. } | Self::Ellipse { .. } | Self::Polygon { .. } => true,
                    _ => false,
                },
                ToolOption::LineCap(..) => match self {
                    Self::Path { .. } => true,
                    _ => false,
                },
                ToolOption::LineJoin(..) => match self {
                    Self::Path { .. } | Self::Polygon { .. } => true,
                    _ => false,
                },
                ToolOption::FillColor(..) => match self {
                    Self::Path { .. } | Self::Ellipse { .. } | Self::Polygon { .. } => true,
                    _ => false,
                },
                ToolOption::NumEdges(..) => match self {
                    Self::Polygon { .. } => true,
                    _ => false,
                },
                ToolOption::ClosedPath(..) => match self {
                    Self::Path { .. } => true,
                    _ => false,
                },
            })
            .collect::<Vec<ToolOption>>()
    }
}

impl Default for ToolState {
    fn default() -> Self {
        Self::new(Tool::Pointer)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ToolOptionHandle {
    LineColor,
    StrokeWidth,
    LineCap,
    LineJoin,
    FillColor,
    NumEdges,
    ClosedPath,
}

#[derive(Clone, Copy, Debug)]
pub enum ToolOption {
    LineColor(Option<ColorU>),
    StrokeWidth(f32),
    LineCap(LineCap),
    LineJoin(LineJoin),
    FillColor(Option<ColorU>),
    NumEdges(u8),
    ClosedPath(bool),
}

impl ToolOption {
    pub fn handle(&self) -> ToolOptionHandle {
        match self {
            Self::LineColor(..) => ToolOptionHandle::LineColor,
            Self::StrokeWidth(..) => ToolOptionHandle::StrokeWidth,
            Self::LineCap(..) => ToolOptionHandle::LineCap,
            Self::LineJoin(..) => ToolOptionHandle::LineJoin,
            Self::FillColor(..) => ToolOptionHandle::FillColor,
            Self::NumEdges(..) => ToolOptionHandle::NumEdges,
            Self::ClosedPath(..) => ToolOptionHandle::ClosedPath,
        }
    }
}

#[derive(Clone, Debug)]
struct Selection {
    objects: HashSet<Uuid>,
}

impl Selection {
    fn clear(&mut self) {
        self.objects.clear();
    }
}

#[derive(Clone, Debug)]
struct Options {
    line_color: Option<ColorU>,
    stroke_width: f32,
    line_cap: LineCap,
    line_join: LineJoin,
    fill_color: Option<ColorU>,
    num_edges: u8,
    closed_path: bool,
}

#[derive(Clone, Debug)]
pub struct EditState {
    tool_state: ToolState,
    options: Options,
    selection: Selection,
}

impl Default for EditState {
    fn default() -> Self {
        EditState {
            tool_state: ToolState::default(),
            //TODO: configure/persist defaults
            options: Options {
                line_color: Some(ColorU::black()),
                stroke_width: 3.0,
                line_cap: LineCap::default(),
                line_join: LineJoin::default(),
                fill_color: Some(ColorU::white()),
                num_edges: 4,
                closed_path: false,
            },
            selection: Selection {
                objects: HashSet::new(),
            },
        }
    }
}

impl EditState {
    pub fn switch_tool(&mut self, tool: &Tool) {
        self.tool_state.switch_tool(*tool);
        //TODO: conditionally clear selection
    }

    pub fn mouse_cursor(&self) -> MouseCursor {
        self.tool_state.mouse_cursor()
    }

    pub fn selection_shape(&self, stage_position: Vector2F) -> SelectionShape {
        self.tool_state.selection_shape(stage_position)
    }

    pub fn on_mouse_event(
        &self,
        mouse_event: MouseEvent,
        stage_position: Vector2F,
        in_bounds: bool,
    ) -> Option<EditMessage> {
        if !in_bounds {
            match mouse_event {
                MouseEvent::Input { state, .. } if state == ButtonState::Pressed => {
                    Some(EditMessage::Cancel)
                }
                _ => None,
            }
        } else {
            let tool_message =
                self.tool_state
                    .on_mouse_event(mouse_event, stage_position, self.tool_options())?;
            Some(EditMessage::ToolUpdate(tool_message))
        }
    }

    fn tool_options(&self) -> Vec<ToolOption> {
        vec![
            ToolOption::LineColor(self.options.line_color),
            ToolOption::StrokeWidth(self.options.stroke_width),
            ToolOption::LineCap(self.options.line_cap),
            ToolOption::LineJoin(self.options.line_join),
            ToolOption::FillColor(self.options.fill_color),
            ToolOption::NumEdges(self.options.num_edges),
            ToolOption::ClosedPath(self.options.closed_path),
        ]
    }

    fn enabled_options(&self) -> HashMap<ToolOptionHandle, ToolOption> {
        self.tool_state
            .get_options(self.tool_options())
            .drain(..)
            .map(|o| (o.handle(), o))
            .collect::<HashMap<ToolOptionHandle, ToolOption>>()
    }

    pub fn update(&mut self, message: &EditMessage) {
        match message {
            EditMessage::ToolUpdate(tool_message) => {
                self.tool_state.update(&tool_message);
            }
            EditMessage::ToolChange(tool) => {
                self.switch_tool(tool);
            }
            EditMessage::Cancel => {
                self.tool_state.cancel_action();
                self.selection.clear();
            }
            EditMessage::ChangeOption(option) => match option {
                ToolOption::LineColor(line_color) => self.options.line_color = *line_color,
                ToolOption::StrokeWidth(stroke_width) => self.options.stroke_width = *stroke_width,
                ToolOption::LineCap(line_cap) => self.options.line_cap = *line_cap,
                ToolOption::LineJoin(line_join) => self.options.line_join = *line_join,
                ToolOption::FillColor(fill_color) => self.options.fill_color = *fill_color,
                ToolOption::NumEdges(num_edges) => self.options.num_edges = *num_edges,
                ToolOption::ClosedPath(closed_path) => self.options.closed_path = *closed_path,
            },
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct EditDisplayState {
    stroke_width: TextInputState,
    num_edges: TextInputState,
}

impl EditDisplayState {
    pub fn options_pane(&mut self, edit_state: &EditState) -> Column<AppMessage> {
        let enabled_options = edit_state.enabled_options();
        let mut column = Column::new().padding(20).spacing(3);
        //Order here is display order. MAYBE TODO: abstract display order?
        if let Some(ToolOption::NumEdges(num_edges)) =
            enabled_options.get(&ToolOptionHandle::NumEdges)
        {
            let num_edges = *num_edges;
            column = column.push(Row::new().push(Text::new("Sides:")).push(TextInput::new(
                &mut self.num_edges,
                "",
                &format!("{}", num_edges),
                move |value| {
                    AppMessage::from_tool_option(ToolOption::NumEdges(
                        value.parse::<u8>().unwrap_or(num_edges),
                    ))
                },
            )))
        }
        if let Some(ToolOption::LineColor(line_color)) =
            enabled_options.get(&ToolOptionHandle::LineColor)
        {}
        if let Some(ToolOption::FillColor(fill_color)) =
            enabled_options.get(&ToolOptionHandle::FillColor)
        {}
        if let Some(ToolOption::StrokeWidth(stroke_width)) =
            enabled_options.get(&ToolOptionHandle::StrokeWidth)
        {
            let stroke_width = *stroke_width;
            column = column.push(
                Row::new()
                    .push(Text::new("Stroke Width:"))
                    .push(TextInput::new(
                        &mut self.stroke_width,
                        "",
                        &format!("{}", stroke_width),
                        move |value| {
                            AppMessage::from_tool_option(ToolOption::StrokeWidth(
                                value.parse::<f32>().unwrap_or(stroke_width),
                            ))
                        },
                    )),
            )
        }
        if let Some(ToolOption::LineCap(line_cap)) = enabled_options.get(&ToolOptionHandle::LineCap)
        {
            //TODO: combobox once Iced supports it
        }
        if let Some(ToolOption::LineJoin(line_join)) =
            enabled_options.get(&ToolOptionHandle::LineJoin)
        {
            //TODO: combobox once Iced supports it
            /*column = column.push(Column::new().push(Text::new("Line Join Method")).push(
                Radio::new(LineJoin::Bevel, "Bevel", Some(*line_join), |bevel| {
                    AppMessage::from_tool_option(ToolOption::LineJoin(bevel))
                }),
            ))*/
        }
        if let Some(ToolOption::ClosedPath(closed_path)) =
            enabled_options.get(&ToolOptionHandle::ClosedPath)
        {
            column = column.push(Checkbox::new(*closed_path, "Close Path", |value| {
                AppMessage::from_tool_option(ToolOption::ClosedPath(value))
            }))
        }
        column
    }
}

/* Shape operations:
   merge,
   split,
   delete,
   add,
   update_color,
   update_stroke (for edge, morph shape issues?),
   change_edge_type (for edge, morth shape issues?)
  Entity operations:
   new,
   delete,
   add_part,
   remove_part,
   update_transform
*/
