#![deny(clippy::all)]
use iced_native::{
    image::Handle as ImageHandle, input::mouse::Button as MouseButton,
    input::mouse::Event as MouseEvent, input::ButtonState, MouseCursor,
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
pub enum ToolMessage {
    PathStart { start_position: Vector2F },
    PathNext { next_position: Vector2F },
    PathPlaceHover { hover_position: Vector2F },
    PathEnd,
}

#[derive(Clone, Copy, Debug)]
enum PlacementState {
    None,
    Placing,
}

#[derive(Clone, Debug)]
enum ToolState {
    Pointer, //grab: edge, point, fill, scale_x, scale_y, scale_xy, entity, group. hover
    Path {
        placement_state: PlacementState,
    },
    Polygon {
        num_edges: u8, //don't support edges over some reasonable size.
        center: Vector2F,
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
    ) -> Option<ToolMessage> {
        //println!("{:?}", stage_position);
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
}

impl Default for ToolState {
    fn default() -> Self {
        Self::new(Tool::Pointer)
    }
}

#[derive(Debug, Clone)]
pub enum EditMessage {
    ToolUpdate(ToolMessage),
    ToolChange(Tool),
    Cancel,
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
pub struct EditState {
    tool_state: ToolState,
    selection: Selection,
}

impl Default for EditState {
    fn default() -> Self {
        EditState {
            tool_state: ToolState::default(),
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
            None
        } else {
            let tool_message = self
                .tool_state
                .on_mouse_event(mouse_event, stage_position)?;
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
        }
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
