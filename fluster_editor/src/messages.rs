use crate::tools::{Tool, ToolOption};
use pathfinder_geometry::vector::Vector2F;
use uuid::Uuid;

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

// TODO: Enum, EntityHandle, PartHandle, UtilityHandle, etc?
#[derive(Clone, Debug)]
pub struct SelectionHandle {
    entity_id: Uuid,
    part_id: Uuid,
    handles: Vec<VertexHandle>,
}

impl SelectionHandle {
    pub fn new(entity_id: Uuid, part_id: Uuid, handles: Vec<VertexHandle>) -> Self {
        Self {
            entity_id,
            part_id,
            handles,
        }
    }

    pub fn has_vertex(&self) -> bool {
        self.handles.len() > 0
    }

    pub fn entity_id(&self) -> &Uuid {
        &self.entity_id
    }

    pub fn min_vertex(&self) -> Option<&VertexHandle> {
        self.handles
            .iter()
            .min_by(|a, b| a.separation.partial_cmp(&b.separation).unwrap())
    }
}

// TODO: differentiate between control point and vertex?
#[derive(Clone, Debug)]
pub struct VertexHandle {
    position: Vector2F,
    vertex_id: usize,
    edge_id: usize,
    library_id: Uuid,
    separation: f32,
}

impl VertexHandle {
    pub fn new(
        library_id: Uuid,
        edge_id: usize,
        vertex_id: usize,
        position: Vector2F,
        separation: f32,
    ) -> Self {
        Self {
            library_id,
            edge_id,
            vertex_id,
            position,
            separation,
        }
    }

    pub fn position(&self) -> &Vector2F {
        &self.position
    }

    pub fn vertex_id(&self) -> usize {
        self.vertex_id
    }

    pub fn edge_id(&self) -> usize {
        self.edge_id
    }

    pub fn library_id(&self) -> &Uuid {
        &self.library_id
    }
}
