use crate::tools::{Tool, ToolOption};
use fluster_core::engine::SelectionHandle;
use pathfinder_geometry::vector::Vector2F;

#[derive(Debug, Clone)]
pub enum AppMessage {
    EditMessage(EditMessage),
    EditHandleMessage(Vec<SelectionHandle>),
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

// TODO: could we use just one End message?
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
    TemplateStart {
        start_position: Vector2F,
        options: Vec<ToolOption>,
        template: Template,
    },
    TemplatePlaceHover {
        hover_position: Vector2F,
    },
    TemplateEnd,
}

#[derive(Copy, Clone, Debug)]
pub enum Template {
    Ellipse,
    Polygon,
    Rectangle,
}
