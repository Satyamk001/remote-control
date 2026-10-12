use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{SharedRb, Producer, Consumer};
use std::sync::Arc;
use tracing::{info, error};

pub struct AudioCapture {
    _stream: cpal::Stream,
    pub consumer: Consumer<f32, Arc<SharedRb<f32, Vec<std::mem::MaybeUninit<f32>>>>>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioCapture {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| "Failed to find default output device for loopback".to_string())?;

        let config = device.default_output_config()
            .map_err(|e| format!("Failed to get output config: {}", e))?;
            
        info!("Audio capture format: {:?}", config);

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        // Ring buffer to pass audio data from the cpal thread to the WebRTC thread
        let ring = SharedRb::new(48000 * 2 * 4); // ~4 seconds of stereo 48kHz
        let (mut producer, consumer) = ring.split();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &_| {
                        let _ = producer.push_slice(data);
                    },
                    |err| error!("Audio capture error: {}", err),
                    None
                )
            },
            _ => {
                // To keep the MVP simple, we only support F32, which is the most common for WASAPI loopback
                return Err("Unsupported audio format. Requires F32.".to_string());
            }
        }.map_err(|e| format!("Failed to build stream: {}", e))?;

        stream.play().map_err(|e| format!("Failed to start stream: {}", e))?;

        Ok(Self {
            _stream: stream,
            consumer,
            sample_rate,
            channels,
        })
    }
}
