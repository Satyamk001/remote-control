#[cfg(target_os = "windows")]
pub mod dxgi;
pub mod error;

use error::CaptureError;

/// Format of the captured pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Bgra8,
    Rgba8,
    Unknown,
}

/// A captured frame with metadata.
#[derive(Debug, Clone)]
pub struct Frame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub timestamp_ms: u64,
    pub monitor_id: u32,
}

/// Abstract interface for screen capture implementations.
pub trait ScreenCapture {
    /// Captures the next frame from the screen.
    fn capture_frame(&mut self) -> Result<Frame, CaptureError>;
    
    /// Returns the current resolution (width, height).
    fn resolution(&self) -> (u32, u32);
    
    /// True if a display change (resolution/monitor layout) was detected.
    fn display_changed(&self) -> bool;

    /// Switch to a different monitor
    fn switch_monitor(&mut self, id: u32) -> Result<(), CaptureError>;
}
