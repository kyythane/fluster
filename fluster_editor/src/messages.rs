use crate::{
    simulation::SelectionHandle,
    tools::{Tool, ToolOption},
};
use pathfinder_geometry::vector::Vector2F;

#[derive(Debug, Clone)]
pub enum AppMessage {
    EditMessage(EditMessage),
    StageUpdateMessage,
}

impl AppMessage {
    pub fn from_edit_message(message: EditMessage) -> Self {
        Self::EditMessage(message)
    }

    pub fn from_tool_option(option: ToolOption) -> Self {
        Self::from_edit_message(EditMessage::ChangeOption(option))
    }
}

#[derive(Debug, Clone)]
pub enum EditMessage {
    ToolUpdate(ToolMessage),
    ChangeOption(ToolOption),
    ToolChange(Tool),
    Cancel,
}

#[derive(Clone, Debug)]
pub enum ToolMessage {
    PathStart {
        start_position: Vector2F,
        options: Vec<ToolOption>,
    },
    PathNext {
        next_position: Vector2F,
    },
    PathPlaceHover {
        hover_position: Vector2F,
    },
    PathEnd,
    MovePointStart {
        selection_handle: SelectionHandle,
    },
    MovePointHover {
        hover_position: Vector2F,
    },
    MovePointEnd,
}
