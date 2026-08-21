use screen_capture::{ScreenCapture, dxgi::WindowsScreenCapture, error::CaptureError};
use std::time::{Instant, Duration};

fn main() {
    println!("Starting Screen Capture Diagnostics...");

    let mut capture = match WindowsScreenCapture::new(0, 0) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to initialize capture: {:?}", e);
            return;
        }
    };

    let (w, h) = capture.resolution();
    println!("Resolution: {}x{}", w, h);

    let mut frames_captured = 0;
    let mut timeouts = 0;
    let start_time = Instant::now();
    let mut last_report = start_time;

    loop {
        let frame_start = Instant::now();
        match capture.capture_frame() {
            Ok(_frame) => {
                frames_captured += 1;
            }
            Err(CaptureError::Timeout) => {
                timeouts += 1;
            }
            Err(e) => {
                eprintln!("Capture error: {:?}", e);
                break;
            }
        }

        let elapsed = frame_start.elapsed();
        if elapsed < Duration::from_millis(16) {
            std::thread::sleep(Duration::from_millis(16) - elapsed);
        }

        if last_report.elapsed() >= Duration::from_secs(1) {
            println!("FPS: {} | Timeouts: {} | Frame time: {:?}", frames_captured, timeouts, elapsed);
            frames_captured = 0;
            timeouts = 0;
            last_report = Instant::now();
        }
        
        if start_time.elapsed() >= Duration::from_secs(5) {
            println!("Diagnostics complete.");
            break;
        }
    }
}
