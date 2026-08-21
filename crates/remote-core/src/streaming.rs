use networking::connection::Connection;
use networking::protocol::Permissions;
use std::io::{self, Write};
use networking::transport::Transport;
use screen_capture::ScreenCapture;
use video::{
    VideoEncoder, VideoDecoder, Renderer,
    packetizer::{VideoPacketizer, VideoDepacketizer},
};
use networking::protocol::{ProtocolMessage, MessagePayload};
use tracing::{info, error, warn};
use std::time::Instant;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub async fn run_host_pipeline<C>(
    mut capture: C,
    mut encoder: Box<dyn VideoEncoder + Send>,
    mut connection: Connection,
) 
where 
    C: ScreenCapture + Send + 'static,
{
    let mut packetizer = VideoPacketizer::new();
    let mut metrics = crate::metrics::MetricsTracker::new();
    
    info!("Starting host streaming pipeline...");

    // Phase 9: Host Approval Prompt
    println!("\n=================================");
    println!("INCOMING REMOTE CONNECTION!");
    println!("Do you want to allow this connection? [y/N]");
    print!("> ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    if input.trim().to_lowercase() != "y" {
        error!("Connection rejected by host.");
        return;
    }

    let mut permissions = Permissions::default();

    println!("Allow Mouse/Keyboard control? [y/N]");
    print!("> ");
    let _ = io::stdout().flush();
    let mut input2 = String::new();
    let _ = io::stdin().read_line(&mut input2);
    if input2.trim().to_lowercase() == "y" {
        permissions.mouse = true;
        permissions.keyboard = true;
    }

    println!("Allow Clipboard sync? [y/N]");
    print!("> ");
    let _ = io::stdout().flush();
    let mut input3 = String::new();
    let _ = io::stdin().read_line(&mut input3);
    if input3.trim().to_lowercase() == "y" {
        permissions.clipboard = true;
    }

    println!("Allow File Transfers? [y/N]");
    print!("> ");
    let _ = io::stdout().flush();
    let mut input4 = String::new();
    let _ = io::stdin().read_line(&mut input4);
    if input4.trim().to_lowercase() == "y" {
        permissions.files = true;
    }
    
    info!("Connection approved. Permissions: {:?}", permissions);
    println!("=================================\n");

    let mut receiver = connection.receiver;
    let sender = connection.sender;
    let permissions_clone = permissions.clone();

    let target_monitor = Arc::new(AtomicU32::new(0));
    let target_monitor_clone = target_monitor.clone();
    let sender_for_receiver = sender.clone();

    // Phase 11: Spawn Audio Capture Thread (Host -> Client)
    if permissions_clone.audio {
        let sender_for_audio = sender.clone();
        std::thread::spawn(move || {
            if let Ok(mut audio) = crate::audio::AudioCapture::new() {
                info!("Audio capture started successfully");
                loop {
                    let mut buf = vec![0.0f32; 1024];
                    let read = audio.consumer.pop_slice(&mut buf);
                    if read > 0 {
                        // Cast f32 slice to u8 slice for transport (quick and dirty MVP)
                        let data: Vec<u8> = bytemuck::cast_slice(&buf[..read]).to_vec();
                        let packet = networking::protocol::ProtocolMessage {
                            seq: 0,
                            timestamp_ms: 0,
                            payload: MessagePayload::AudioPacket { data },
                        };
                        let _ = sender_for_audio.blocking_send(packet);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            } else {
                error!("Failed to start audio capture");
            }
        });
    }

    // Spawn the receiver loop for incoming DataChannel messages (Input, Clipboard, Files, Monitor)
    tokio::spawn(async move {
        let mut clipboard_manager = crate::clipboard::ClipboardManager::new();
        // Use a generic downloads folder for now
        let downloads_path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")).join("downloads");
        std::fs::create_dir_all(&downloads_path).unwrap_or_default();
        let mut file_receiver = crate::transfer::FileReceiver::new(downloads_path);

        while let Some(msg) = receiver.recv().await {
            match msg.payload {
                MessagePayload::InputEvent { .. } => {
                    if !permissions_clone.mouse && !permissions_clone.keyboard {
                        warn!("Blocked unauthorized input event");
                    } else {
                        // TODO: Handle input (Phase 11)
                    }
                }
                MessagePayload::ClipboardData { text } => {
                    if !permissions_clone.clipboard {
                        warn!("Blocked unauthorized clipboard write");
                    } else {
                        info!("Received clipboard text: {}", text);
                        clipboard_manager.set_text(&text);
                    }
                }
                MessagePayload::FileOffer { file_id, filename, size } => {
                    if !permissions_clone.files {
                        warn!("Blocked unauthorized file transfer offer");
                    } else {
                        if let Err(e) = file_receiver.accept_offer(&file_id, &filename, size) {
                            error!("Failed to accept file offer: {:?}", e);
                        }
                    }
                }
                MessagePayload::FileChunk { file_id, seq, data, checksum } => {
                    if permissions_clone.files {
                        if let Err(e) = file_receiver.receive_chunk(&file_id, seq, &data, checksum) {
                            error!("Failed to process file chunk: {:?}", e);
                        }
                    }
                }
                MessagePayload::FileComplete { file_id } => {
                    if permissions_clone.files {
                        if let Err(e) = file_receiver.complete_transfer(&file_id) {
                            error!("Failed to complete file transfer: {:?}", e);
                        }
                    }
                }
                MessagePayload::FileCancel { file_id, reason } => {
                    if permissions_clone.files {
                        warn!("File transfer cancelled: {}", reason);
                        file_receiver.cancel_transfer(&file_id);
                    }
                }
                MessagePayload::GetMonitors => {
                    #[cfg(windows)]
                    {
                        let monitors = screen_capture::dxgi::WindowsScreenCapture::list_monitors();
                        let reply = ProtocolMessage {
                            seq: 0, timestamp_ms: 0,
                            payload: MessagePayload::MonitorList { monitors },
                        };
                        let _ = sender_for_receiver.send(reply).await;
                    }
                }
                MessagePayload::SwitchMonitor { index } => {
                    info!("Client requested monitor switch to {}", index);
                    target_monitor_clone.store(index as u32, Ordering::Relaxed);
                }
                _ => {}
            }
        }
    });
    
    info!("Connection approved. Permissions: {:?}", permissions);
    println!("=================================\n");
    
    let mut current_monitor = 0;

    loop {
        let target = target_monitor.load(Ordering::Relaxed);
        if target != current_monitor {
            info!("Host switching capture to monitor {}", target);
            if let Err(e) = capture.switch_monitor(target) {
                error!("Failed to switch monitor: {:?}", e);
            } else {
                current_monitor = target;
            }
        }

        if metrics.should_throttle() {
            // Adaptive bitrate: drop capture rate to 30 FPS instead of 60 FPS
            std::thread::sleep(std::time::Duration::from_millis(33)); // 30 FPS throttle
        } else {
            std::thread::sleep(std::time::Duration::from_millis(16)); // 60 FPS target
        }

        metrics.start_frame();
        
        // 1. Capture
        let frame = match capture.capture_frame() {
            Ok(f) => {
                metrics.record_capture();
                f
            }
            Err(screen_capture::error::CaptureError::Timeout) => continue,
            Err(e) => {
                error!("Capture error: {:?}", e);
                break;
            }
        };

        // 2. Encode
        let enc_start = Instant::now();
        let encoded = match encoder.encode(&frame) {
            Ok(e) => {
                metrics.record_encode(enc_start);
                e
            }
            Err(e) => {
                error!("Encode error: {:?}", e);
                break;
            }
        };

        // 3. Packetize
        let packets = packetizer.packetize(&encoded).unwrap();

        // 4. Transport (Send)
        for packet in packets {
            if sender.send(packet).await.is_err() {
                error!("Connection closed by remote.");
                return;
            }
        }
        
        metrics.log_metrics();
    }
}

pub async fn run_client_pipeline<D, R>(
    mut connection: Connection,
    mut decoder: D,
    mut renderer: R,
)
where 
    D: VideoDecoder + Send + 'static,
    R: Renderer + Send + 'static,
{
    let mut depacketizer = VideoDepacketizer::new();
    
    info!("Starting client streaming pipeline...");
    
    while let Some(msg) = connection.receiver.recv().await {
        // 1. Transport (Receive) handled by `connection`

        // 2. Depacketize
        let encoded_opt = match depacketizer.depacketize(&msg) {
            Ok(opt) => opt,
            Err(e) => {
                error!("Depacketize error: {:?}", e);
                continue;
            }
        };

        if let Some(encoded) = encoded_opt {
            // 3. Decode
            let dec_start = Instant::now();
            let frame = match decoder.decode(&encoded) {
                Ok(f) => f,
                Err(e) => {
                    error!("Decode error: {:?}", e);
                    continue;
                }
            };
            let dec_latency = dec_start.elapsed();

            // 4. Render
            let rend_start = Instant::now();
            if let Err(e) = renderer.render(&frame) {
                error!("Render error: {:?}", e);
            }
            let rend_latency = rend_start.elapsed();
            
            if encoded.frame_id % 60 == 0 {
                info!("Client: Dec: {:?}, Rend: {:?}", dec_latency, rend_latency);
            }
        }
    }
}
