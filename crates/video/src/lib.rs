pub mod error;
pub mod packetizer;
pub mod encoder;
pub mod h264;
pub mod jpeg;
pub mod decoder;
pub mod renderer;

use error::VideoError;
use screen_capture::Frame;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct EncodedFrame {
    pub frame_id: u32,
    pub timestamp_ms: u64,
    pub data: Vec<u8>,
    pub is_keyframe: bool,
}

pub trait VideoEncoder {
    fn encode(&mut self, frame: &Frame) -> Result<EncodedFrame, VideoError>;
}

pub trait VideoDecoder {
    fn decode(&mut self, encoded_frame: &EncodedFrame) -> Result<Frame, VideoError>;
}

pub trait Renderer {
    fn render(&mut self, frame: &Frame) -> Result<(), VideoError>;
}

#[derive(Debug, Default)]
pub struct Diagnostics {
    pub fps: f64,
    pub bitrate_kbps: f64,
    pub packet_loss_percent: f64,
    pub capture_latency: Duration,
    pub encode_latency: Duration,
    pub network_latency: Duration,
    pub decode_latency: Duration,
    pub render_latency: Duration,
    pub total_latency: Duration,
}
