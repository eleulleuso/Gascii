/// Frame data structure for video frames
#[derive(Clone)]
pub struct FrameData {
    pub buffer: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl FrameData {
    pub fn new(buffer: Vec<u8>, width: u32, height: u32) -> Self {
        Self { buffer, width, height }
    }
}
