use std::time::Duration;

#[derive(Default, Copy, Clone, Debug)]
pub struct FrameTime {
    pub delta_time: Duration,
    pub delta_frame: u32,
    // Other frame time data will *eventually* live here
}
