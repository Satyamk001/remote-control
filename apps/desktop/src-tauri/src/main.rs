#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use networking::signaling::SignalingClient;
use tracing::{info, error};
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tauri::Manager;
use webrtc::track::track_local::TrackLocalWriter;
use std::sync::atomic::{AtomicUsize, Ordering};

static BYTES_SENT: AtomicUsize = AtomicUsize::new(0);
static BYTES_RECEIVED: AtomicUsize = AtomicUsize::new(0);
static CURRENT_BITRATE: AtomicUsize = AtomicUsize::new(0);
static CURRENT_LATENCY: AtomicUsize = AtomicUsize::new(42);
/// Viewer-controlled audio mute. When true, the host stops shipping audio PCM.
static AUDIO_MUTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Host-measured per-stage telemetry (microseconds / packed values).
static ENCODE_LATENCY_US: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_LATENCY_US: AtomicUsize = AtomicUsize::new(0);
static RTT_JITTER_MS: AtomicUsize = AtomicUsize::new(0);
/// Current capture resolution, packed as (width << 16) | height.
static CURRENT_RESOLUTION: AtomicUsize = AtomicUsize::new(0);
/// Viewer-reported decode latency (microseconds), set via report_decode_latency.
static DECODE_LATENCY_US: AtomicUsize = AtomicUsize::new(0);
/// Monotonically increasing session/capture generation. Each capture thread
/// records the generation it was started with and exits once the global value
/// advances (a new session started or the current one was terminated). This
/// guarantees the DXGI duplication is released before the next session tries to
/// acquire it — otherwise the new session fails and falls back to the mock
/// gradient generator.
static CAPTURE_GENERATION: AtomicUsize = AtomicUsize::new(0);

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum DataPacket {
    Input(input::InputEvent),
    Clipboard(String),
    SwitchMonitor(u8),
    Disconnect,
    /// Latency probe sent by the host. `nonce` round-trips so stale replies are
    /// ignored; `ts_micros` is the host's send time (monotonic epoch micros).
    Ping { nonce: u64, ts_micros: u64 },
    /// Echo of a `Ping`, sent by the viewer, carrying the original fields back.
    Pong { nonce: u64, ts_micros: u64 },
    /// Viewer -> host: mute/unmute system audio capture.
    ToggleAudio { muted: bool },
    /// Viewer -> host: begin a file upload. `size` is total bytes.
    FileOffer { file_id: String, filename: String, size: u64 },
    /// Viewer -> host: a sequential chunk of the file, CRC32-checked.
    FileChunk { file_id: String, seq: u64, data: Vec<u8>, checksum: u32 },
    /// Viewer -> host: finalize the file (rename .part -> final).
    FileComplete { file_id: String },
}

/// Monotonic-ish timestamp in microseconds since the UNIX epoch, used for RTT.
fn now_micros() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}

/// Frame an encoded H.264 access unit for the viewer's binary stream.
///
/// Wire format (little-endian): `[u8 flags][u32 payload_len][payload]`.
/// `flags` bit0 = keyframe (IDR). This lets the viewer's WebCodecs decoder
/// split the continuous byte stream back into discrete `EncodedVideoChunk`s and
/// tag key vs delta frames without parsing the Annex-B start codes itself.
fn frame_h264(payload: &[u8], is_keyframe: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(5 + payload.len());
    out.push(if is_keyframe { 1u8 } else { 0u8 });
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// Pick a JPEG quality + inter-frame delay (ms) from the measured RTT.
///
/// Higher RTT => lower quality and lower frame rate, trading fidelity for
/// responsiveness and bandwidth. Tuned around the project's existing 100ms
/// throttle threshold (`remote-core::metrics`). Returns `(quality, frame_delay_ms)`.
fn adaptive_params(rtt_ms: usize) -> (u8, u64) {
    match rtt_ms {
        0..=40 => (80, 16),   // great link: high quality, ~60 FPS cap
        41..=100 => (70, 33), // normal: ~30 FPS
        101..=200 => (55, 50), // congested: ~20 FPS
        _ => (40, 100),        // poor: ~10 FPS, low quality
    }
}

/// Fallback animated test pattern used when native DXGI capture is unavailable
/// (e.g. WSL/Linux builds). Encodes synthetic BGRA frames through the same
/// H.264 encoder so the wire format matches the real path, keeping the viewer
/// pipeline exercisable without a real desktop capture backend.
fn run_mock_capture(
    frame_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    switch_rx: std::sync::mpsc::Receiver<u32>,
) {
    use screen_capture::{Frame, PixelFormat};
    let (w, h): (u32, u32) = (320, 180);
    let mut encoder = match video::h264::H264Encoder::new() {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Mock H264 encoder init failed: {}", e);
            return;
        }
    };
    let mut x_shift: u8 = 0;
    loop {
        // Drain any switch requests so the channel doesn't back up.
        while switch_rx.try_recv().is_ok() {}

        // Build a synthetic BGRA test pattern.
        let mut data = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                data[i] = y as u8; // B
                data[i + 1] = 100; // G
                data[i + 2] = x_shift.wrapping_add(x as u8); // R
                data[i + 3] = 255; // A
            }
        }
        x_shift = x_shift.wrapping_add(5);

        let frame = Frame {
            data,
            width: w,
            height: h,
            pixel_format: PixelFormat::Bgra8,
            timestamp_ms: now_micros() / 1000,
            monitor_id: 0,
        };

        if let Ok(encoded) = video::VideoEncoder::encode(&mut encoder, &frame) {
            if !encoded.data.is_empty() {
                let wire = frame_h264(&encoded.data, encoded.is_keyframe);
                if frame_tx.blocking_send(wire).is_err() {
                    break;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(33));
    }
}

struct AppSession {
    webrtc: tokio::sync::RwLock<Option<std::sync::Arc<webrtc_lib::transport::WebRtcTransport>>>,
    pending_offer: tokio::sync::RwLock<Option<(String, String)>>,
    pending_ice: tokio::sync::RwLock<Vec<webrtc::ice_transport::ice_candidate::RTCIceCandidateInit>>,
    auto_accept: tokio::sync::RwLock<bool>,
}

/// Signaling client state. Managed synchronously at startup (as `None`) so
/// commands never hit Tauri's "state not managed" panic if invoked before the
/// background WebSocket connect finishes. The connect task fills in the client.
type SignalingState = std::sync::Arc<tokio::sync::RwLock<Option<networking::signaling::SignalingClient>>>;

#[tauri::command]
async fn get_devices(client: tauri::State<'_, SignalingState>) -> Result<Vec<serde_json::Value>, String> {
    let guard = client.read().await;
    if let Some(c) = guard.as_ref() {
        c.request_hosts_update().await;
        // Give it a tiny bit of time to fetch
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let hosts = c.hosts.read().await;
        let mut devices = Vec::new();
        for (i, host) in hosts.iter().enumerate() {
            let _ = i;
            devices.push(serde_json::json!({
                "id": host.device_id, // we use deviceId as the connection ID for the UI
                "session_id": host.session_id,
                "name": format!("Host-{}", &host.device_id[..8]),
                "status": "online",
                "last_seen": "Active now"
            }));
        }
        Ok(devices)
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
async fn get_diagnostics() -> Result<serde_json::Value, String> {
    let bitrate_kbps = CURRENT_BITRATE.load(Ordering::Relaxed);
    let latency_ms = CURRENT_LATENCY.load(Ordering::Relaxed);
    let (_q, delay_ms) = adaptive_params(latency_ms);
    let fps = if delay_ms > 0 { 1000 / delay_ms } else { 0 };

    let encode_us = ENCODE_LATENCY_US.load(Ordering::Relaxed);
    let capture_us = CAPTURE_LATENCY_US.load(Ordering::Relaxed);
    let decode_us = DECODE_LATENCY_US.load(Ordering::Relaxed);
    let jitter_ms = RTT_JITTER_MS.load(Ordering::Relaxed);
    let res = CURRENT_RESOLUTION.load(Ordering::Relaxed);
    let (w, h) = ((res >> 16) & 0xFFFF, res & 0xFFFF);
    let resolution = if w > 0 && h > 0 {
        format!("{}x{}", w, h)
    } else {
        "—".to_string()
    };

    let fmt_us = |us: usize| -> String {
        if us == 0 { "—".to_string() } else { format!("{:.1}ms", us as f64 / 1000.0) }
    };

    Ok(serde_json::json!({
        "fps": fps, // adaptive, derived from live RTT
        "bitrate": format!("{:.2} Mbps", bitrate_kbps as f64 / 1000.0),
        "rtt": format!("{}ms", latency_ms),
        // SCTP DataChannels are reliable (retransmitted), so there is no
        // application-visible packet loss to report.
        "packet_loss": "0% (reliable)",
        "jitter": format!("{}ms", jitter_ms),
        "capture_latency": fmt_us(capture_us),
        "encode_latency": fmt_us(encode_us),
        "decode_latency": fmt_us(decode_us),
        "resolution": resolution,
        "codec": "H.264 (OpenH264)",
        "network": "WebRTC P2P (DataChannel)",
        // Legacy field kept for the top status pill.
        "latency": format!("{}ms", latency_ms)
    }))
}

/// Viewer -> host: report measured WebCodecs decode latency (microseconds) so
/// the diagnostics HUD can show a real end-to-end picture.
#[tauri::command]
fn report_decode_latency(micros: u32) -> Result<(), String> {
    DECODE_LATENCY_US.store(micros as usize, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
async fn connect_to_device(
    id: String, 
    client: tauri::State<'_, SignalingState>,
    session: tauri::State<'_, AppSession>
) -> Result<serde_json::Value, String> {
    info!("Connecting to device: {}", id);
    let c = client.read().await.clone().ok_or("No signaling client")?;
    let c = &c;
    
    // Find session_id from id (device_id)
    let hosts = c.hosts.read().await;
    let target = hosts.iter().find(|h| h.device_id == id).ok_or("Host not found")?.clone();
    let session_id = target.session_id;
    drop(hosts);

    info!("Found session {} for device {}", session_id, id);

    c.send_join_session(&session_id).await;

    let (ice_tx, mut ice_rx) = tokio::sync::mpsc::channel::<webrtc::ice_transport::ice_candidate::RTCIceCandidateInit>(32);
    let (data_tx, mut data_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
    let (video_tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(16);
    let (audio_tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(64);
    
    let c_clone = c.clone();
    let sid = session_id.clone();
    
    tokio::spawn(async move {
        while let Some(candidate) = ice_rx.recv().await {
            c_clone.send_ice_candidate(&sid, &candidate.candidate, candidate.sdp_mid, candidate.sdp_mline_index).await;
        }
    });

    let transport = webrtc_lib::transport::WebRtcTransport::new(false, ice_tx, data_tx, video_tx.clone(), audio_tx.clone()).await.map_err(|e| e.to_string())?;
    
    // Client Data Loop
    let dc = transport.data_channel.clone();
    tokio::spawn(async move {
        let mut clip = remote_core::clipboard::ClipboardManager::new();
        let mut last_text = clip.get_text().unwrap_or_default();
        loop {
            tokio::select! {
                Some(bytes) = data_rx.recv() => {
                    if let Ok(packet) = bincode::deserialize::<DataPacket>(&bytes) {
                        match packet {
                            DataPacket::Clipboard(text) => {
                                clip.set_text(&text);
                                last_text = text;
                            }
                            DataPacket::Ping { nonce, ts_micros } => {
                                // Echo immediately so the host can compute RTT.
                                if let Some(channel) = &dc {
                                    let pong = DataPacket::Pong { nonce, ts_micros };
                                    if let Ok(bytes) = bincode::serialize(&pong) {
                                        let _ = channel.send(&bytes.into()).await;
                                    }
                                }
                            }
                            _ => { info!("Client received packet: {:?}", packet); }
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                    if let Some(text) = clip.get_text() {
                        if text != last_text {
                            last_text = text.clone();
                            if let Some(channel) = &dc {
                                let packet = DataPacket::Clipboard(text);
                                if let Ok(bytes) = bincode::serialize(&packet) {
                                    let _ = channel.send(&bytes.into()).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    
    // Spawn the H.264 binary loopback server for the viewer.
    //
    // The data channel delivers already-framed access units
    // (`[u8 flags][u32 len][payload]`, see `frame_h264`). We relay those raw
    // bytes over a single long-lived HTTP response; the frontend reads the body
    // as a stream and feeds each access unit to a WebCodecs VideoDecoder.
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
    let local_addr = listener.local_addr().map_err(|e| e.to_string())?;
    let stream_url = format!("http://{}/stream", local_addr);
    
    tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut rx = video_tx.subscribe();
                tokio::spawn(async move {
                    // Read + discard the HTTP request line/headers.
                    let mut buf = [0; 1024];
                    let _ = socket.read(&mut buf).await;

                    // application/octet-stream: a continuous binary body of
                    // length-prefixed H.264 access units.
                    let headers = "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n";
                    if socket.write_all(headers.as_bytes()).await.is_err() {
                        return;
                    }

                    while let Ok(frame) = rx.recv().await {
                        // `frame` is already a framed access unit; relay verbatim.
                        if socket.write_all(&frame).await.is_err() {
                            break;
                        }
                        BYTES_RECEIVED.fetch_add(frame.len(), Ordering::Relaxed);
                    }
                });
            }
        }
    });

    // Spawn the audio PCM loopback server for the viewer.
    //
    // The audio channel delivers raw interleaved stereo f32 LE PCM samples
    // (48kHz, see the host audio loop). We relay them over a long-lived HTTP
    // response; the frontend reads the body as a stream and plays it via the
    // Web Audio API.
    let audio_listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
    let audio_addr = audio_listener.local_addr().map_err(|e| e.to_string())?;
    let audio_url = format!("http://{}/audio", audio_addr);

    tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = audio_listener.accept().await {
                let mut rx = audio_tx.subscribe();
                tokio::spawn(async move {
                    let mut buf = [0; 1024];
                    let _ = socket.read(&mut buf).await;
                    let headers = "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n";
                    if socket.write_all(headers.as_bytes()).await.is_err() {
                        return;
                    }
                    while let Ok(chunk) = rx.recv().await {
                        if socket.write_all(&chunk).await.is_err() {
                            break;
                        }
                        BYTES_RECEIVED.fetch_add(chunk.len(), Ordering::Relaxed);
                    }
                });
            }
        }
    });

    if let Ok(offer) = transport.peer_connection.create_offer(None).await {
        if let Ok(_) = transport.peer_connection.set_local_description(offer.clone()).await {
            c.send_sdp_offer(&session_id, &offer.sdp).await;
            
            *session.webrtc.write().await = Some(std::sync::Arc::new(transport));
            return Ok(serde_json::json!({ "video": stream_url, "audio": audio_url }));
        }
    }
    
    Err("Failed to create offer".into())
}

#[tauri::command]
async fn send_file(
    filename: String,
    data: Vec<u8>,
    session: tauri::State<'_, AppSession>,
) -> Result<(), String> {
    use std::sync::Arc;
    // Snapshot the data channel so we don't hold the session lock while sending.
    let dc = {
        let guard = session.webrtc.read().await;
        let transport = guard.as_ref().ok_or("No active session")?;
        transport.data_channel.clone().ok_or("No data channel")?
    };

    let file_id = format!("{}-{}", now_micros(), filename.len());
    let size = data.len() as u64;

    // Offer.
    let offer = DataPacket::FileOffer {
        file_id: file_id.clone(),
        filename: filename.clone(),
        size,
    };
    let bytes = bincode::serialize(&offer).map_err(|e| e.to_string())?;
    dc.send(&bytes.into()).await.map_err(|e| e.to_string())?;

    // Chunks (16KB), each CRC32-checked, in sequence.
    const CHUNK: usize = 16 * 1024;
    let dc2: Arc<_> = dc.clone();
    for (seq, chunk) in data.chunks(CHUNK).enumerate() {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(chunk);
        let checksum = hasher.finalize();
        let packet = DataPacket::FileChunk {
            file_id: file_id.clone(),
            seq: seq as u64,
            data: chunk.to_vec(),
            checksum,
        };
        let bytes = bincode::serialize(&packet).map_err(|e| e.to_string())?;
        dc2.send(&bytes.into()).await.map_err(|e| e.to_string())?;
    }

    // Complete.
    let complete = DataPacket::FileComplete { file_id };
    let bytes = bincode::serialize(&complete).map_err(|e| e.to_string())?;
    dc2.send(&bytes.into()).await.map_err(|e| e.to_string())?;

    info!("Sent file '{}' ({} bytes) over data channel.", filename, size);
    Ok(())
}

#[tauri::command]
async fn toggle_audio(muted: bool, session: tauri::State<'_, AppSession>) -> Result<(), String> {
    let packet = DataPacket::ToggleAudio { muted };
    let bytes = bincode::serialize(&packet).map_err(|e| e.to_string())?;
    if let Some(transport) = session.webrtc.read().await.as_ref() {
        if let Some(dc) = &transport.data_channel {
            let _ = dc.send(&bytes.into()).await;
        }
    }
    Ok(())
}

#[tauri::command]
async fn switch_monitor(index: u8, session: tauri::State<'_, AppSession>) -> Result<(), String> {    info!("Switching monitor to {}", index);
    let packet = DataPacket::SwitchMonitor(index);
    let bytes = bincode::serialize(&packet).map_err(|e| e.to_string())?;
    
    if let Some(transport) = session.webrtc.read().await.as_ref() {
        if let Some(dc) = &transport.data_channel {
            let _ = dc.send(&bytes.into()).await;
        }
    }
    Ok(())
}

#[tauri::command]
fn get_local_device_id(state: tauri::State<'_, remote_core::security::Identity>) -> String {
    let bytes = state.verifying_key.to_bytes();
    hex::encode(bytes).to_uppercase()
}

#[tauri::command]
async fn get_host_session_status(session: tauri::State<'_, AppSession>) -> Result<serde_json::Value, String> {
    let mut is_dead = false;
    if let Some(transport) = session.webrtc.read().await.as_ref() {
        use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
        let state = transport.peer_connection.ice_connection_state();
        
        if state == RTCIceConnectionState::Failed || state == RTCIceConnectionState::Closed || state == RTCIceConnectionState::Disconnected {
            is_dead = true;
        } else {
            let bitrate_kbps = CURRENT_BITRATE.load(Ordering::Relaxed);
            let latency_ms = CURRENT_LATENCY.load(Ordering::Relaxed);
            
            return Ok(serde_json::json!({
                "state": "active",
                "client_name": "Remote Viewer",
                "duration": "Active",
                "network": "WebRTC P2P",
                "fps": 30,
                "bitrate": format!("{:.2} Mbps", bitrate_kbps as f64 / 1000.0),
                "latency": format!("{}ms", latency_ms)
            }));
        }
    }
    
    if is_dead {
        *session.webrtc.write().await = None;
    }
    
    if session.pending_offer.read().await.is_some() {
        return Ok(serde_json::json!({
            "state": "awaiting_approval",
            "client_name": "Incoming Connection"
        }));
    }

    Ok(serde_json::json!({
        "state": "idle"
    }))
}


#[tauri::command]
async fn toggle_auto_accept(enabled: bool, session: tauri::State<'_, AppSession>) -> Result<(), String> {
    *session.auto_accept.write().await = enabled;
    Ok(())
}

#[tauri::command]
async fn get_auto_accept(session: tauri::State<'_, AppSession>) -> Result<bool, String> {
    Ok(*session.auto_accept.read().await)
}

#[tauri::command]
async fn approve_connection(
    allowed: bool, 
    session: tauri::State<'_, AppSession>,
    client: tauri::State<'_, SignalingState>
) -> Result<(), String> {
    info!("Host connection approval: {}", allowed);
    if !allowed {
        *session.pending_offer.write().await = None;
        session.pending_ice.write().await.clear();
        return Ok(());
    }

    let offer_data = session.pending_offer.write().await.take();
    if let Some((session_id, sdp)) = offer_data {
        let c = client.read().await.clone().ok_or("No signaling client")?;
        process_offer_internal(session.inner(), c, session_id, sdp).await?;
    }
    
    Ok(())
}

async fn process_offer_internal(
    session: &AppSession,
    c: networking::signaling::SignalingClient,
    session_id: String,
    sdp: String
) -> Result<(), String> {
        // Stop any prior session's capture thread and release its DXGI
        // duplication before starting a new one. Without this, a reconnect
        // finds the duplication still held and falls back to the mock gradient.
        let my_gen = CAPTURE_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
        if let Some(old) = session.webrtc.write().await.take() {
            let _ = old.peer_connection.close().await;
        }
        // Give the old blocking capture thread a moment to observe the
        // generation change and release DXGI.
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        let (ice_tx, mut ice_rx) = tokio::sync::mpsc::channel::<webrtc::ice_transport::ice_candidate::RTCIceCandidateInit>(32);
        let (data_tx, mut data_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
        let (video_tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(16);
        let (audio_tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(64);
        
        let c_clone = c.clone();
        let sid = session_id.clone();
        
        tokio::spawn(async move {
            while let Some(candidate) = ice_rx.recv().await {
                c_clone.send_ice_candidate(&sid, &candidate.candidate, candidate.sdp_mid, candidate.sdp_mline_index).await;
            }
        });
        
        if let Ok(transport) = webrtc_lib::transport::WebRtcTransport::new(true, ice_tx, data_tx, video_tx, audio_tx).await {
            let vc = transport.video_channel.clone();
            let ac = transport.audio_channel.clone();

            // Channel used to forward monitor-switch requests from the data loop
            // to the (blocking) capture loop. std::sync::mpsc so it can be polled
            // from the spawn_blocking capture thread without async.
            let (switch_tx, switch_rx) = std::sync::mpsc::channel::<u32>();

            // Host Data Loop (Injector & Clipboard)
            let dc = transport.data_channel.clone();
            let pc = transport.peer_connection.clone();
            tokio::spawn(async move {
                use input::WindowsInputInjector;
                let injector = input::create_injector();
                let mut clip = remote_core::clipboard::ClipboardManager::new();
                let mut last_text = clip.get_text().unwrap_or_default();
                let mut ping_nonce: u64 = 0;

                // Incoming file uploads land in a `downloads` folder next to the
                // working directory, with path-traversal protection in the
                // FileReceiver.
                let downloads = std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .join("downloads");
                let _ = std::fs::create_dir_all(&downloads);
                let mut file_rx = remote_core::transfer::FileReceiver::new(downloads);
                
                loop {
                    tokio::select! {
                        Some(bytes) = data_rx.recv() => {
                            if let Ok(packet) = bincode::deserialize::<DataPacket>(&bytes) {
                                match packet {
                                    DataPacket::Input(event) => {
                                        let _ = injector.inject_event(&event);
                                    }
                                    DataPacket::Clipboard(text) => {
                                        clip.set_text(&text);
                                        last_text = text;
                                    }
                                    DataPacket::SwitchMonitor(index) => {
                                        info!("Host received SwitchMonitor({})", index);
                                        let _ = switch_tx.send(index as u32);
                                    }
                                    DataPacket::Pong { ts_micros, .. } => {
                                        // Round-trip time = now - original send time.
                                        let rtt_us = now_micros().saturating_sub(ts_micros);
                                        let rtt_ms = (rtt_us / 1000) as usize;
                                        // Smooth with a simple EWMA to avoid jitter
                                        // whipsawing the adaptive controller.
                                        let prev = CURRENT_LATENCY.load(Ordering::Relaxed);
                                        let smoothed = (prev * 3 + rtt_ms) / 4;
                                        CURRENT_LATENCY.store(smoothed, Ordering::Relaxed);
                                        // Jitter: EWMA of the absolute RTT delta
                                        // (RFC3550-style interarrival jitter).
                                        let delta = rtt_ms.abs_diff(prev);
                                        let pj = RTT_JITTER_MS.load(Ordering::Relaxed);
                                        RTT_JITTER_MS.store((pj * 7 + delta) / 8, Ordering::Relaxed);
                                    }
                                    DataPacket::Disconnect => {
                                        info!("Host received Disconnect packet, closing session.");
                                        let _ = pc.close().await;
                                        break;
                                    }
                                    DataPacket::ToggleAudio { muted } => {
                                        info!("Host received ToggleAudio(muted={})", muted);
                                        AUDIO_MUTED.store(muted, Ordering::Relaxed);
                                    }
                                    DataPacket::FileOffer { file_id, filename, size } => {
                                        info!("File offer: {} ({} bytes)", filename, size);
                                        if let Err(e) = file_rx.accept_offer(&file_id, &filename, size) {
                                            error!("Rejected file offer {}: {:?}", filename, e);
                                        }
                                    }
                                    DataPacket::FileChunk { file_id, seq, data, checksum } => {
                                        if let Err(e) = file_rx.receive_chunk(&file_id, seq, &data, checksum) {
                                            error!("File chunk {} seq {} failed: {:?}", file_id, seq, e);
                                            file_rx.cancel_transfer(&file_id);
                                        }
                                    }
                                    DataPacket::FileComplete { file_id } => {
                                        match file_rx.complete_transfer(&file_id) {
                                            Ok(_) => info!("File {} received successfully.", file_id),
                                            Err(e) => error!("File complete {} failed: {:?}", file_id, e),
                                        }
                                    }
                                    DataPacket::Ping { .. } => { /* host doesn't expect pings */ }
                                }
                            }
                        }
                        _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                            // Send a latency probe each tick.
                            if let Some(channel) = &dc {
                                ping_nonce = ping_nonce.wrapping_add(1);
                                let ping = DataPacket::Ping { nonce: ping_nonce, ts_micros: now_micros() };
                                if let Ok(bytes) = bincode::serialize(&ping) {
                                    let _ = channel.send(&bytes.into()).await;
                                }
                            }
                            if let Some(text) = clip.get_text() {
                                if text != last_text {
                                    last_text = text.clone();
                                    if let Some(channel) = &dc {
                                        let packet = DataPacket::Clipboard(text);
                                        if let Ok(bytes) = bincode::serialize(&packet) {
                                            let _ = channel.send(&bytes.into()).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });
            
            // Host Video Capture Loop (real DXGI desktop capture -> JPEG).
            //
            // The DXGI capture object holds COM interface pointers that are not
            // safe to hold across .await points, so the capture + encode runs on a
            // dedicated blocking thread. Encoded JPEG frames are forwarded to an
            // async task over a std::sync::mpsc channel, which performs the actual
            // WebRTC data-channel send.
            if let Some(channel) = vc {
                // Async sender: receives finished JPEG frames and ships them.
                let (frame_tx, mut frame_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
                tokio::spawn(async move {
                    while let Some(bytes) = frame_rx.recv().await {
                        if channel.send(&bytes.clone().into()).await.is_ok() {
                            BYTES_SENT.fetch_add(bytes.len(), Ordering::Relaxed);
                        }
                    }
                });

                // Blocking capture + encode thread.
                tokio::task::spawn_blocking(move || {
                    // Give the data channel a moment to open before streaming.
                    std::thread::sleep(std::time::Duration::from_millis(500));

                    tracing::info!(
                        "Initializing DXGI Desktop Duplication API for live capture..."
                    );

                    #[cfg(target_os = "windows")]
                    {
                        use screen_capture::{PixelFormat, ScreenCapture};

                        match screen_capture::dxgi::WindowsScreenCapture::new(0, 0) {
                            Ok(mut capture) => {
                                let (w, h) = capture.resolution();
                                tracing::info!("DXGI capture active at {}x{}", w, h);

                                let mut encoder = match video::h264::H264Encoder::new() {
                                    Ok(e) => e,
                                    Err(e) => {
                                        tracing::error!(
                                            "H264 encoder init failed ({}); falling back to mock.",
                                            e
                                        );
                                        run_mock_capture(frame_tx, switch_rx);
                                        return;
                                    }
                                };
                                // Cache the most recent frame so we can re-emit
                                // keyframes while the screen is static (DXGI
                                // returns Timeout instead of a frame when nothing
                                // changes, which would otherwise starve a
                                // mid-stream viewer of keyframes).
                                let mut last_frame: Option<screen_capture::Frame> = None;
                                loop {
                                    // Exit if a newer session started (releases DXGI for it).
                                    if CAPTURE_GENERATION.load(Ordering::Relaxed) != my_gen {
                                        tracing::info!("Capture gen {} superseded; stopping capture.", my_gen);
                                        break;
                                    }
                                    // Apply any pending monitor-switch request (take the latest).
                                    let mut target: Option<u32> = None;
                                    while let Ok(idx) = switch_rx.try_recv() {
                                        target = Some(idx);
                                    }
                                    if let Some(idx) = target {
                                        tracing::info!("Switching DXGI capture to monitor {}", idx);
                                        if let Err(e) = capture.switch_monitor(idx) {
                                            tracing::warn!("switch_monitor failed: {}", e);
                                        }
                                    }

                                    let cap_start = std::time::Instant::now();
                                    match capture.capture_frame() {
                                        Ok(frame) => {
                                            CAPTURE_LATENCY_US.store(
                                                cap_start.elapsed().as_micros() as usize,
                                                Ordering::Relaxed,
                                            );
                                            CURRENT_RESOLUTION.store(
                                                ((frame.width as usize) << 16) | (frame.height as usize),
                                                Ordering::Relaxed,
                                            );
                                            last_frame = Some(frame);
                                            let f = last_frame.as_ref().unwrap();
                                            let enc_start = std::time::Instant::now();
                                            match video::VideoEncoder::encode(&mut encoder, f) {
                                                Ok(encoded) if !encoded.data.is_empty() => {
                                                    ENCODE_LATENCY_US.store(
                                                        enc_start.elapsed().as_micros() as usize,
                                                        Ordering::Relaxed,
                                                    );
                                                    let wire = frame_h264(
                                                        &encoded.data,
                                                        encoded.is_keyframe,
                                                    );
                                                    if frame_tx.blocking_send(wire).is_err() {
                                                        break; // receiver gone -> session ended
                                                    }
                                                }
                                                Ok(_) => { /* encoder buffered, no output yet */ }
                                                Err(e) => {
                                                    tracing::error!("H264 encode error: {}", e);
                                                }
                                            }
                                        }
                                        Err(screen_capture::error::CaptureError::Timeout) => {
                                            // Static screen: DXGI returned no new frame. Re-encode
                                            // the last frame so the encoder's periodic keyframe
                                            // timer still fires and mid-stream viewers can sync.
                                            if let Some(f) = last_frame.as_ref() {
                                                if let Ok(encoded) =
                                                    video::VideoEncoder::encode(&mut encoder, f)
                                                {
                                                    if encoded.is_keyframe && !encoded.data.is_empty() {
                                                        let wire = frame_h264(
                                                            &encoded.data,
                                                            encoded.is_keyframe,
                                                        );
                                                        if frame_tx.blocking_send(wire).is_err() {
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(screen_capture::error::CaptureError::DisplayChanged) => {
                                            tracing::warn!(
                                                "Display changed; re-initializing DXGI capture."
                                            );
                                            if let Ok(fresh) =
                                                screen_capture::dxgi::WindowsScreenCapture::new(0, 0)
                                            {
                                                capture = fresh;
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Capture error: {}", e);
                                            std::thread::sleep(std::time::Duration::from_millis(100));
                                        }
                                    }

                                    // Adaptive frame-rate cap derived from live
                                    // RTT. DXGI only returns changed frames, so
                                    // this is an upper bound, not a busy loop.
                                    let rtt = CURRENT_LATENCY.load(Ordering::Relaxed);
                                    let (_q, delay_ms) = adaptive_params(rtt);
                                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Native DXGI capture failed ({}). Falling back to mock generator.",
                                    e
                                );
                                run_mock_capture(frame_tx, switch_rx);
                            }
                        }

                        // Suppress unused warning when PixelFormat path is compiled.
                        let _ = PixelFormat::Bgra8;
                    }

                    #[cfg(not(target_os = "windows"))]
                    {
                        tracing::warn!(
                            "Non-Windows build: DXGI unavailable, using mock generator."
                        );
                        run_mock_capture(frame_tx, switch_rx);
                    }
                });
            }

            // Host Audio Capture Loop (WASAPI loopback PCM -> audio channel).
            //
            // cpal's Stream is not Send, so capture lives on a dedicated blocking
            // thread that polls the lock-free ring buffer and forwards raw
            // interleaved stereo f32 LE PCM to an async task for the actual send.
            if let Some(channel) = ac {
                let (pcm_tx, mut pcm_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
                tokio::spawn(async move {
                    while let Some(bytes) = pcm_rx.recv().await {
                        if channel.send(&bytes.clone().into()).await.is_ok() {
                            BYTES_SENT.fetch_add(bytes.len(), Ordering::Relaxed);
                        }
                    }
                });

                tokio::task::spawn_blocking(move || {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    match remote_core::audio::AudioCapture::new() {
                        Ok(mut capture) => {
                            tracing::info!("WASAPI loopback audio capture active.");

                            // Advertise the real capture format to the viewer as
                            // the first message: magic "RCAF" + u32 LE sample_rate
                            // + u16 LE channels. The viewer configures its
                            // AudioContext from this instead of assuming 48kHz/2ch.
                            let mut header = Vec::with_capacity(10);
                            header.extend_from_slice(b"RCAF");
                            header.extend_from_slice(&capture.sample_rate.to_le_bytes());
                            header.extend_from_slice(&capture.channels.to_le_bytes());
                            let _ = pcm_tx.blocking_send(header);

                            let mut scratch = vec![0f32; 4096];
                            loop {
                                // Drop audio while muted to honor the viewer toggle
                                // and the permission model.
                                if AUDIO_MUTED.load(Ordering::Relaxed) {
                                    // Keep the ring from overflowing while muted.
                                    while capture.consumer.pop().is_some() {}
                                    std::thread::sleep(std::time::Duration::from_millis(20));
                                    continue;
                                }

                                let n = capture.consumer.pop_slice(&mut scratch);
                                if n == 0 {
                                    std::thread::sleep(std::time::Duration::from_millis(5));
                                    continue;
                                }

                                // Serialize f32 samples as little-endian bytes.
                                let mut bytes = Vec::with_capacity(n * 4);
                                for &s in &scratch[..n] {
                                    bytes.extend_from_slice(&s.to_le_bytes());
                                }
                                if pcm_tx.blocking_send(bytes).is_err() {
                                    break; // viewer gone -> session ended
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Audio capture unavailable: {}", e);
                        }
                    }
                });
            }

            let offer = webrtc::peer_connection::sdp::session_description::RTCSessionDescription::offer(sdp).unwrap();
            if let Ok(_) = transport.peer_connection.set_remote_description(offer).await {
                if let Ok(answer) = transport.peer_connection.create_answer(None).await {
                    if let Ok(_) = transport.peer_connection.set_local_description(answer.clone()).await {
                        c.send_sdp_answer(&session_id, &answer.sdp).await;
                        
                        *session.webrtc.write().await = Some(std::sync::Arc::new(transport));
                        
                        let mut pending_ice = session.pending_ice.write().await;
                        if let Some(t) = session.webrtc.read().await.as_ref() {
                            for ice in pending_ice.drain(..) {
                                let _ = t.peer_connection.add_ice_candidate(ice).await;
                            }
                        }
                    }
                }
            }
        }
    Ok(())
}

#[tauri::command]
async fn send_input_event(event: input::InputEvent, session: tauri::State<'_, AppSession>) -> Result<(), String> {
    let packet = DataPacket::Input(event);
    let bytes = bincode::serialize(&packet).map_err(|e| e.to_string())?;
    
    if let Some(transport) = session.webrtc.read().await.as_ref() {
        if let Some(dc) = &transport.data_channel {
            let _ = dc.send(&bytes.into()).await;
        }
    }
    Ok(())
}

#[tauri::command]
async fn terminate_host_session(session: tauri::State<'_, AppSession>) -> Result<(), String> {
    info!("Terminating active session");
    // Stop the capture thread (releases DXGI) by advancing the generation.
    CAPTURE_GENERATION.fetch_add(1, Ordering::Relaxed);
    if let Some(transport) = session.webrtc.write().await.take() {
        if let Some(dc) = &transport.data_channel {
            let packet = DataPacket::Disconnect;
            if let Ok(bytes) = bincode::serialize(&packet) {
                let _ = dc.send(&bytes.into()).await;
            }
        }
        let _ = transport.peer_connection.close().await;
    }
    *session.pending_offer.write().await = None;
    session.pending_ice.write().await.clear();
    Ok(())
}

fn main() {
    // Logging: write to BOTH stdout (visible when launched from a console in
    // debug builds) and a log file. The release build uses the Windows GUI
    // subsystem (no console), so the file is the reliable way to read logs.
    //
    // Log location: <REMOTE_CONTROL_DATA_DIR or %TEMP%>/remote-control.log
    let log_dir = std::env::var("REMOTE_CONTROL_DATA_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("remote-control.log");

    match std::fs::File::create(&log_path) {
        Ok(file) => {
            // File layer (no ANSI). We log the path to stdout too, in case a
            // console is attached.
            tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(file))
                .init();
            tracing::info!("Logging to {}", log_path.display());
        }
        Err(_) => {
            // Fall back to stdout-only if the file can't be created.
            tracing_subscriber::fmt::init();
        }
    }

    tauri::Builder::default()
        .setup(|app| {
            // Data directory holds the persistent device identity (identity.key)
            // and other per-install state. It can be overridden with
            // REMOTE_CONTROL_DATA_DIR so a SECOND instance on the same machine
            // gets its own Device ID — required for the single-PC loopback
            // self-test (one instance as host, one as viewer).
            let app_data_dir = match std::env::var("REMOTE_CONTROL_DATA_DIR") {
                Ok(dir) if !dir.is_empty() => std::path::PathBuf::from(dir),
                _ => app
                    .path_resolver()
                    .app_data_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from(".")),
            };
            if !app_data_dir.exists() {
                std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");
            }
            
            let identity = remote_core::security::Identity::load_or_generate(app_data_dir);
            let device_id_hex = hex::encode(identity.verifying_key.to_bytes()).to_uppercase();
            app.manage(identity);
            app.manage(AppSession { 
                webrtc: tokio::sync::RwLock::new(None),
                pending_offer: tokio::sync::RwLock::new(None),
                pending_ice: tokio::sync::RwLock::new(Vec::new()),
                auto_accept: tokio::sync::RwLock::new(false),
            });
            // Manage the signaling state up front (as None) so commands never
            // panic on unmanaged state before the connect task completes.
            let signaling_state: SignalingState = std::sync::Arc::new(tokio::sync::RwLock::new(None));
            app.manage(signaling_state.clone());
            
            // Start Telemetry Loop
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    let sent = BYTES_SENT.swap(0, Ordering::Relaxed);
                    let recv = BYTES_RECEIVED.swap(0, Ordering::Relaxed);
                    let total_bits = (sent + recv) * 8;
                    let kbps = total_bits / 1000;
                    CURRENT_BITRATE.store(kbps, Ordering::Relaxed);

                    // CURRENT_LATENCY is now populated by the real ping/pong RTT
                    // probe in the host data loop (DataPacket::Ping/Pong), not a
                    // synthetic value.
                }
            });
            
            // Start Signaling Client in background.
            //
            // The signaling endpoint can be overridden at runtime via the
            // REMOTE_CONTROL_SIGNALING_URL environment variable. This keeps the
            // deployed Render.com instance as the zero-config default while
            // allowing self-hosted / LAN signaling servers for private testing.
            let signaling_url = std::env::var("REMOTE_CONTROL_SIGNALING_URL")
                .unwrap_or_else(|_| "wss://remote-control-y1jb.onrender.com".to_string());
            let app_handle = app.handle();
            let signaling_state_bg = signaling_state.clone();
            tauri::async_runtime::spawn(async move {
                match networking::signaling::SignalingClient::connect(&signaling_url, &device_id_hex).await {
                    Ok(client) => {
                        info!("Connected to signaling server as {}", device_id_hex);
                        
                        let mut events_rx = client.subscribe_events();
                        let client_clone = client.clone();
                        let app_handle_clone = app_handle.clone();
                        
                        tauri::async_runtime::spawn(async move {
                            while let Ok(event) = events_rx.recv().await {
                                match event {
                                    networking::signaling::SignalingEvent::SdpOffer { session_id, sdp } => {
                                        info!("Host received SDP Offer for session: {}", session_id);
                                        let state = app_handle_clone.state::<AppSession>();
                                        let is_auto = *state.auto_accept.read().await;
                                        if is_auto {
                                            info!("Auto-accepting connection request...");
                                            let c = client_clone.clone();
                                            let _ = process_offer_internal(state.inner(), c, session_id, sdp).await;
                                        } else {
                                            *state.pending_offer.write().await = Some((session_id, sdp));
                                        }
                                    }
                                    networking::signaling::SignalingEvent::SdpAnswer { session_id, sdp } => {
                                        info!("Client received SDP Answer for session: {}", session_id);
                                        let transport_opt = app_handle_clone.state::<AppSession>().webrtc.read().await.clone();
                                        if let Some(transport) = transport_opt {
                                            let answer = webrtc::peer_connection::sdp::session_description::RTCSessionDescription::answer(sdp).unwrap();
                                            let _ = transport.peer_connection.set_remote_description(answer).await;
                                        }
                                    }
                                    networking::signaling::SignalingEvent::IceCandidate { session_id: _, candidate, sdp_mid, sdp_m_line_index } => {
                                        info!("Received remote ICE candidate");
                                        let transport_opt = app_handle_clone.state::<AppSession>().webrtc.read().await.clone();
                                        let init = webrtc::ice_transport::ice_candidate::RTCIceCandidateInit {
                                            candidate,
                                            sdp_mid,
                                            sdp_mline_index: sdp_m_line_index,
                                            username_fragment: None,
                                        };
                                        if let Some(transport) = transport_opt {
                                            let _ = transport.peer_connection.add_ice_candidate(init).await;
                                        } else {
                                            app_handle_clone.state::<AppSession>().pending_ice.write().await.push(init);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        });

                        *signaling_state_bg.write().await = Some(client);
                    }
                    Err(e) => {
                        error!("Failed to connect to signaling server: {}", e);
                        // Leave the managed state as None; commands handle it.
                    }
                }
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_devices,
            get_diagnostics,
            report_decode_latency,
            connect_to_device,
            switch_monitor,
            send_input_event,
            toggle_audio,
            send_file,
            get_local_device_id,
            get_host_session_status,
            approve_connection,
            terminate_host_session,
            toggle_auto_accept,
            get_auto_accept
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
