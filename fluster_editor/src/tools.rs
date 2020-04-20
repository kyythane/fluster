#![deny(clippy::all)]
use fluster_core::types::shapes::Edge;
use pathfinder_color::ColorU;
use pathfinder_geometry::vector::Vector2F;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub enum Tool {
    Pointer,
    Line,
    Curve,
    Polygon,
    Ellipse,
    Fill,
    Eyedropper,
}

pub struct MouseState {
    position: Vector2F,
}

impl MouseState {
    pub fn position(&self) -> Vector2F {
        self.position()
    }
}

enum PlacementState {
    None,
    Placing,
    Placed,
}

enum ToolState {
    Pointer, //grab: edge, point, fill, scale_x, scale_y, scale_xy, entity, group. hover
    Line {
        //TODO: color
        start: Vector2F,
        end: Vector2F,
        path: Vec<Edge>,
        placement_state: PlacementState,
    },
    Curve {
        placement_state: PlacementState,
        path: Vec<Edge>,
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
            Tool::Pointer => ToolState::Pointer,
            Tool::Line => ToolState::Line {
                start: Vector2F::zero(),
                end: Vector2F::zero(),
                path: vec![],
                placement_state: PlacementState::None,
            },
            _ => unimplemented!(),
        }
    }

    fn switch_tool(self, tool: Tool) -> Self {
        match tool {
            Tool::Pointer => ToolState::Pointer,
            Tool::Line => match self {
                ToolState::Line { path, .. } => ToolState::Line {
                    start: Vector2F::zero(),
                    end: Vector2F::zero(),
                    path,
                    placement_state: PlacementState::None,
                },
                ToolState::Curve { path, .. } => ToolState::Line {
                    start: Vector2F::zero(),
                    end: Vector2F::zero(),
                    path,
                    placement_state: PlacementState::None,
                },
                _ => Self::new(tool),
            },
            _ => unimplemented!(),
        }
    }

    fn tool(&self) -> Tool {
        match self {
            ToolState::Pointer => Tool::Pointer,
            ToolState::Line { .. } => Tool::Line,
            ToolState::Curve { .. } => Tool::Curve,
            ToolState::Polygon { .. } => Tool::Polygon,
            ToolState::Ellipse { .. } => Tool::Ellipse,
            ToolState::Fill { .. } => Tool::Fill,
            ToolState::Eyedropper { .. } => Tool::Eyedropper,
        }
    }

    fn cancel_action(self) -> Self {
        ToolState::new(self.tool())
    }
}

struct Selection {
    objects: HashSet<Uuid>,
}

impl Selection {
    fn clear(&mut self) {
        self.objects.clear();
    }
}

pub struct EditState {
    tool_state: ToolState,
    selection: Selection,
}

impl EditState {
    pub fn init() -> Self {
        EditState {
            tool_state: ToolState::new(Tool::Pointer),
            selection: Selection {
                objects: HashSet::new(),
            },
        }
    }

    pub fn cancel_action(self) -> Self {
        let mut state = self;
        state.selection.clear();
        EditState {
            tool_state: state.tool_state.cancel_action(),
            selection: state.selection,
        }
    }

    pub fn switch_tool(self, tool: Tool) -> Self {
        let tool_state = self.tool_state.switch_tool(tool);
        EditState {
            tool_state,
            selection: self.selection,
        }
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
