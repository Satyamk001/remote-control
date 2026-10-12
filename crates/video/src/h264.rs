//! Real H.264 software encoding via OpenH264 (Cisco).
//!
//! Takes BGRA frames from the DXGI capturer and produces Annex-B H.264 NAL
//! units. OpenH264 emits SPS/PPS followed by IDR/P frames; the first encode and
//! every forced keyframe carry the parameter sets needed to (re)initialize a
//! decoder. This is the software path — a future hardware path (NVENC/QuickSync
//! via Media Foundation) can implement the same `VideoEncoder` trait.

use crate::{error::VideoError, EncodedFrame, VideoEncoder};
use openh264::encoder::{Encoder, FrameType};
use openh264::formats::{BgraSliceU8, YUVBuffer};
use screen_capture::Frame;

pub struct H264Encoder {
    encoder: Encoder,
    frame_id: u32,
    width: u32,
    height: u32,
}

impl H264Encoder {
    /// Create a new software H.264 encoder. OpenH264 picks up the resolution
    /// from the first frame and re-initializes automatically if it changes.
    pub fn new() -> Result<Self, VideoError> {
        let encoder = Encoder::new().map_err(|e| VideoError::EncodeError(e.to_string()))?;
        tracing::info!("OpenH264 software encoder initialized.");
        Ok(Self {
            encoder,
            frame_id: 0,
            width: 0,
            height: 0,
        })
    }

    /// Force the next encoded frame to be an IDR keyframe. Used when a new
    /// viewer attaches and needs SPS/PPS + an intra frame to start decoding.
    pub fn request_keyframe(&mut self) {
        self.encoder.force_intra_frame();
    }
}

impl VideoEncoder for H264Encoder {
    fn encode(&mut self, frame: &Frame) -> Result<EncodedFrame, VideoError> {
        let w = frame.width as usize;
        let h = frame.height as usize;
        if w == 0 || h == 0 || frame.data.len() < w * h * 4 {
            return Err(VideoError::EncodeError(format!(
                "invalid frame {}x{} ({} bytes)",
                frame.width,
                frame.height,
                frame.data.len()
            )));
        }

        self.width = frame.width;
        self.height = frame.height;

        // DXGI delivers BGRA8; OpenH264's BgraSliceU8 handles the channel order
        // and converts to the I420 the encoder expects.
        let bgra = BgraSliceU8::new(&frame.data[..w * h * 4], (w, h));
        let yuv = YUVBuffer::from_rgb_source(bgra);

        let bitstream = self
            .encoder
            .encode(&yuv)
            .map_err(|e| VideoError::EncodeError(e.to_string()))?;

        let is_keyframe = matches!(
            bitstream.frame_type(),
            FrameType::IDR | FrameType::I
        );

        let data = bitstream.to_vec();
        self.frame_id = self.frame_id.wrapping_add(1);

        Ok(EncodedFrame {
            frame_id: self.frame_id,
            timestamp_ms: frame.timestamp_ms,
            data,
            is_keyframe,
        })
    }
}
