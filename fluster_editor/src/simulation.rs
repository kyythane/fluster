#![deny(clippy::all)]
use fluster_core::types::model::DisplayLibraryItem;
use iced_native::{MouseCursor, Point};
use pathfinder_color::ColorU;
use std::collections::HashMap;
use uuid::Uuid;

//TODO: Should this be StageState? FlusterState? FrameState is v limited
pub struct FrameState<'b> {
    background_color: ColorU,
    library: &'b HashMap<Uuid, DisplayLibraryItem>,
    //TODO: mouse_state???
}

impl<'a> FrameState<'a> {
    pub fn background_color(&self) -> ColorU {
        self.background_color
    }
    pub fn library(&self) -> &HashMap<Uuid, DisplayLibraryItem> {
        self.library
    }
    //TODO: mouse picking in fluster_core
    pub fn compute_mouse_state(&self, _cursor_position: Point) -> MouseCursor {
        MouseCursor::Pointer
    }
}
