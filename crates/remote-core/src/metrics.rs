use std::time::{Instant, Duration};
use tracing::info;

pub struct MetricsTracker {
    capture_start: Instant,
    pub capture_latency: Duration,
    pub encode_latency: Duration,
    pub network_rtt: Duration,
    pub frames_processed: u64,
    pub dropped_frames: u64,
}

impl MetricsTracker {
    pub fn new() -> Self {
        Self {
            capture_start: Instant::now(),
            capture_latency: Duration::ZERO,
            encode_latency: Duration::ZERO,
            network_rtt: Duration::from_millis(10), // Mock initial RTT
            frames_processed: 0,
            dropped_frames: 0,
        }
    }

    pub fn start_frame(&mut self) {
        self.capture_start = Instant::now();
    }

    pub fn record_capture(&mut self) {
        self.capture_latency = self.capture_start.elapsed();
    }

    pub fn record_encode(&mut self, start: Instant) {
        self.encode_latency = start.elapsed();
        self.frames_processed += 1;
    }

    pub fn log_metrics(&self) {
        if self.frames_processed > 0 && self.frames_processed % 60 == 0 {
            info!(
                "Metrics | Cap: {}ms | Enc: {}ms | RTT: {}ms | Drops: {}",
                self.capture_latency.as_millis(),
                self.encode_latency.as_millis(),
                self.network_rtt.as_millis(),
                self.dropped_frames
            );
        }
    }

    pub fn should_throttle(&self) -> bool {
        // If encode is taking longer than 15ms or RTT > 100ms, throttle
        self.encode_latency.as_millis() > 15 || self.network_rtt.as_millis() > 100
    }
}
