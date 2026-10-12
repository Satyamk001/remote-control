//! JPEG (MJPEG) fallback encoder.
//!
//! Used for viewers whose webview cannot decode H.264 via WebCodecs (notably
//! WebKitGTK on Linux). Each captured BGRA frame is converted to a standalone
//! JPEG so the viewer can display it in an `<img>` via a multipart MJPEG
//! stream. This is more bandwidth-hungry than H.264 but universally renderable.

use crate::{error::VideoError, EncodedFrame, VideoEncoder};
use screen_capture::{Frame, PixelFormat};

pub struct JpegEncoder {
    quality: u8,
    frame_id: u32,
}

impl JpegEncoder {
    pub fn new(quality: u8) -> Self {
        Self { quality, frame_id: 0 }
    }
}

impl VideoEncoder for JpegEncoder {
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

        // Convert the 4-byte source pixels to packed RGB for the JPEG encoder.
        let mut rgb = vec![0u8; w * h * 3];
        match frame.pixel_format {
            PixelFormat::Bgra8 | PixelFormat::Unknown => {
                for i in 0..(w * h) {
                    let s = i * 4;
                    let d = i * 3;
                    rgb[d] = frame.data[s + 2]; // R <- B position
                    rgb[d + 1] = frame.data[s + 1]; // G
                    rgb[d + 2] = frame.data[s]; // B <- R position
                }
            }
            PixelFormat::Rgba8 => {
                for i in 0..(w * h) {
                    let s = i * 4;
                    let d = i * 3;
                    rgb[d] = frame.data[s];
                    rgb[d + 1] = frame.data[s + 1];
                    rgb[d + 2] = frame.data[s + 2];
                }
            }
        }

        let img: image::RgbImage = image::ImageBuffer::from_raw(frame.width, frame.height, rgb)
            .ok_or_else(|| VideoError::EncodeError("ImageBuffer::from_raw failed".into()))?;
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, self.quality)
            .encode_image(&img)
            .map_err(|e| VideoError::EncodeError(e.to_string()))?;

        self.frame_id = self.frame_id.wrapping_add(1);
        Ok(EncodedFrame {
            frame_id: self.frame_id,
            timestamp_ms: frame.timestamp_ms,
            data: cursor.into_inner(),
            // Every JPEG is a complete standalone image (self-contained keyframe).
            is_keyframe: true,
        })
    }
}
