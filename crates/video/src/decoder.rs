use crate::{EncodedFrame, VideoDecoder, error::VideoError};
use screen_capture::{Frame, PixelFormat};

pub struct RawDecoder {
    width: u32,
    height: u32,
}

impl RawDecoder {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl VideoDecoder for RawDecoder {
    fn decode(&mut self, encoded_frame: &EncodedFrame) -> Result<Frame, VideoError> {
        // Raw passthrough for MVP architecture
        Ok(Frame {
            data: encoded_frame.data.clone(),
            width: self.width,
            height: self.height,
            pixel_format: PixelFormat::Bgra8,
            timestamp_ms: encoded_frame.timestamp_ms,
            monitor_id: 0,
        })
    }
}
