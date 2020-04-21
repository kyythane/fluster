#![deny(clippy::all)]
use fluster_core::types::shapes::Edge;
use iced_native::{
    input::mouse::Button as MouseButton, input::mouse::Event as MouseEvent, input::ButtonState,
    MouseCursor, Point,
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

#[derive(Clone, Copy, Debug)]
pub enum ToolMessage {
    PathStart { start_position: Vector2F },
    PathNext { next_position: Vector2F },
    PathEnd { end_position: Vector2F },
}

#[derive(Clone, Copy, Debug)]
enum PlacementState {
    None,
    Placing,
}

#[derive(Clone, Debug)]
enum ToolState {
    Pointer, //grab: edge, point, fill, scale_x, scale_y, scale_xy, entity, group. hover
    //TODO: rename to Path???
    Path {
        //TODO: color
        start: Vector2F,
        end: Vector2F,
        path: Vec<Edge>,
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
                start: Vector2F::zero(),
                end: Vector2F::zero(),
                path: vec![],
                placement_state: PlacementState::None,
            },
            _ => todo!(),
        }
    }

    fn switch_tool(&mut self, tool: Tool) -> Self {
        let previous = mem::take(self);
        match tool {
            Tool::Pointer => Self::Pointer,
            Tool::Path => match previous {
                Self::Path { path, .. } => Self::Path {
                    start: Vector2F::zero(),
                    end: Vector2F::zero(),
                    path,
                    placement_state: PlacementState::None,
                },
                _ => Self::new(tool),
            },
            _ => todo!(),
        }
    }

    fn cancel_action(&self) -> Self {
        Self::new(self.tool())
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

    fn mouse_cursor(&self) -> MouseCursor {
        match self {
            Self::Pointer => MouseCursor::Idle,
            _ => match self.placement_state() {
                PlacementState::None => MouseCursor::Pointer,
                PlacementState::Placing => MouseCursor::Grab,
            },
        }
    }

    fn on_mouse_event(
        &self,
        mouse_event: MouseEvent,
        stage_position: Vector2F,
        in_bounds: bool,
    ) -> Option<ToolMessage> {
        match self {
            Self::Path {
                start,
                end,
                placement_state,
                path,
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
                        PlacementState::Placing => Some(ToolMessage::PathEnd {
                            end_position: stage_position,
                        }),
                    }
                }
                _ => None,
            },
            _ => todo!(),
        }
    }
}

impl Default for ToolState {
    fn default() -> Self {
        Self::Pointer
    }
}

#[derive(Debug, Clone)]
pub enum EditMessage {
    ToolUpdate(ToolMessage),
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
    pub fn cancel_action(&mut self) {
        self.tool_state = self.tool_state.cancel_action();
        self.selection.clear();
    }

    pub fn switch_tool(&mut self, tool: Tool) {
        let tool_state = self.tool_state.switch_tool(tool);
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
        let tool_message =
            self.tool_state
                .on_mouse_event(mouse_event, stage_position, in_bounds)?;
        Some(EditMessage::ToolUpdate(tool_message))
    }

    //pub fn use_tool(self, mouse: MouseState) -> (ToolResult, Self) {}
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
