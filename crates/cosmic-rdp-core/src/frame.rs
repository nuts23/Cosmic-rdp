use std::sync::Arc;

/// A 32-bit RGBA/BGRA video frame received from the remote RDP desktop
#[derive(Debug, Clone, PartialEq)]
pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    /// Raw pixel data in RGBA format (4 bytes per pixel)
    pub data: Arc<[u8]>,
    /// Frame sequence index / timestamp
    pub sequence: u64,
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32, data: Vec<u8>, sequence: u64) -> Self {
        let stride = width * 4;
        Self {
            width,
            height,
            stride,
            data: Arc::from(data.into_boxed_slice()),
            sequence,
        }
    }

    /// Create an empty placeholder frame with a solid background color
    pub fn placeholder(width: u32, height: u32, r: u8, g: u8, b: u8) -> Self {
        let total_bytes = (width * height * 4) as usize;
        let mut data = Vec::with_capacity(total_bytes);
        for _ in 0..(width * height) {
            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255); // Alpha
        }
        Self::new(width, height, data, 0)
    }
}
