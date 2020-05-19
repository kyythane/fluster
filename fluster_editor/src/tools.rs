#![deny(clippy::all)]
use crate::messages::{AppMessage, EditMessage, ToolMessage};
use iced::{Checkbox, Column, Element, Row, Text, TextInput};
use iced_native::{
    image::Handle as ImageHandle, input::mouse::Button as MouseButton,
    input::mouse::Event as MouseEvent, input::ButtonState, text_input::State as TextInputState,
    MouseCursor,
};
use pathfinder_color::ColorU;
use pathfinder_geometry::vector::Vector2F;
use std::collections::HashSet;
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

//TODO: invoke_action
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

    fn on_mouse_event(
        &self,
        mouse_event: MouseEvent,
        stage_position: Vector2F,
        tool_options: &Vec<ToolOption>,
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

    fn get_options(&self, tool_options: &Vec<ToolOption>) -> Vec<ToolOption> {
        tool_options
            .iter()
            .filter(|option| match option {
                ToolOption::LineColor(..) => match self {
                    Self::Path { .. } | Self::Ellipse { .. } | Self::Polygon { .. } => true,
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
                ToolOption::StrokeWidth(..) => match self {
                    Self::Path { .. } | Self::Ellipse { .. } | Self::Polygon { .. } => true,
                    _ => false,
                },
                ToolOption::ClosedPath(..) => match self {
                    Self::Path { .. } => true,
                    _ => false,
                },
            })
            .map(|o| *o)
            .collect::<Vec<ToolOption>>()
    }
}

impl Default for ToolState {
    fn default() -> Self {
        Self::new(Tool::Pointer)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ToolOption {
    LineColor(Option<ColorU>),
    FillColor(Option<ColorU>),
    NumEdges(u8),
    StrokeWidth(f32),
    ClosedPath(bool),
}

impl ToolOption {
    pub fn display_name(&self) -> &str {
        match self {
            Self::LineColor(..) => "Line Color",
            Self::FillColor(..) => "Fille Color",
            Self::NumEdges(..) => "Sides",
            Self::StrokeWidth(..) => "Stroke Width",
            Self::ClosedPath(..) => "Close Shape",
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

#[derive(Clone, Debug, Default)]
struct EditOptionUiState {
    stroke_width: TextInputState,
}

#[derive(Clone, Debug)]
pub struct EditState {
    tool_state: ToolState,
    tool_options: Vec<ToolOption>,
    option_states: EditOptionUiState,
    selection: Selection,
}

impl Default for EditState {
    fn default() -> Self {
        EditState {
            tool_state: ToolState::default(),
            //TODO: configure/persist defaults
            tool_options: vec![
                ToolOption::LineColor(Some(ColorU::black())),
                ToolOption::FillColor(Some(ColorU::white())),
                ToolOption::StrokeWidth(3.0),
                ToolOption::NumEdges(4),
                ToolOption::ClosedPath(false),
            ],
            option_states: EditOptionUiState::default(),
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

    pub fn on_mouse_event(
        &self,
        mouse_event: MouseEvent,
        stage_position: Vector2F,
        in_bounds: bool,
    ) -> Option<EditMessage> {
        if !in_bounds {
            Some(EditMessage::Cancel)
        } else {
            let tool_message =
                self.tool_state
                    .on_mouse_event(mouse_event, stage_position, &self.tool_options)?;
            Some(EditMessage::ToolUpdate(tool_message))
        }
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
            EditMessage::ChangeOption(option) => todo!(),
        }
    }

    pub fn tool_options(&self) -> Vec<ToolOption> {
        self.tool_state.get_options(&self.tool_options)
    }

    pub fn options_pane(&mut self) -> Column<AppMessage> {
        let mut column = Column::new().padding(20).spacing(3);
        for option in self.tool_options() {
            let option_value: Element<AppMessage> = match option {
                ToolOption::LineColor(color) => todo!(),
                ToolOption::FillColor(color) => todo!(),
                ToolOption::NumEdges(edges) => todo!(),
                ToolOption::StrokeWidth(width) => TextInput::new(
                    &mut self.option_states.stroke_width,
                    "",
                    &format!("{}", width),
                    move |value| {
                        AppMessage::from_tool_option(ToolOption::StrokeWidth(
                            value.parse::<f32>().unwrap_or(width),
                        ))
                    },
                )
                .into(),
                ToolOption::ClosedPath(closed) => Checkbox::new(closed, "", |value| {
                    AppMessage::from_tool_option(ToolOption::ClosedPath(value))
                })
                .into(),
            };
            column = column.push(
                Row::new()
                    .push(Text::new(option.display_name()))
                    .push(option_value),
            );
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
