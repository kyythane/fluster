#[macro_use]
extern crate neon_serde;
use neon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

/*thread_local! {
    static fluster_renderer: FlusterEditor = {

    };

}*/

#[derive(Deserialize, Serialize)]
pub struct Frame {
    start_index: u32,
    end_index: u32,
}

#[derive(Deserialize, Serialize)]
pub struct ScrollPosition {
    x: f32,
    y: f32,
}

export! {
    fn initialize() -> () {

    }

    fn set_zoom_position(zoom: f32, scroll: ScrollPosition) -> () {

    }

    fn render_frame(frame_index: u32) -> ByteBuf {
        unimplemented!()
    }
}
