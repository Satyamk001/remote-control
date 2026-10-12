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

#[tauri::command]
async fn get_devices(client: tauri::State<'_, Option<networking::signaling::SignalingClient>>) -> Result<Vec<serde_json::Value>, String> {
    if let Some(c) = client.inner() {
        c.request_hosts_update().await;
        // Give it a tiny bit of time to fetch
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let hosts = c.hosts.read().await;
        let mut devices = Vec::new();
        for (i, host) in hosts.iter().enumerate() {
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
    
    Ok(serde_json::json!({
        "fps": fps, // adaptive, derived from live RTT
        "latency": format!("{}ms", latency_ms),
        "bitrate": format!("{:.2} Mbps", bitrate_kbps as f64 / 1000.0),
        "packetLoss": "0%" // SCTP DataChannels are reliable, so packet loss is handled at the protocol layer
    }))
}

#[tauri::command]
async fn connect_to_device(
    id: String, 
    client: tauri::State<'_, Option<networking::signaling::SignalingClient>>,
    session: tauri::State<'_, AppSession>
) -> Result<String, String> {
    info!("Connecting to device: {}", id);
    let c = client.inner().as_ref().ok_or("No signaling client")?;
    
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
    
    let c_clone = c.clone();
    let sid = session_id.clone();
    
    tokio::spawn(async move {
        while let Some(candidate) = ice_rx.recv().await {
            c_clone.send_ice_candidate(&sid, &candidate.candidate, candidate.sdp_mid, candidate.sdp_mline_index).await;
        }
    });

    let transport = webrtc_lib::transport::WebRtcTransport::new(false, ice_tx, data_tx, video_tx.clone()).await.map_err(|e| e.to_string())?;
    
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

    if let Ok(offer) = transport.peer_connection.create_offer(None).await {
        if let Ok(_) = transport.peer_connection.set_local_description(offer.clone()).await {
            c.send_sdp_offer(&session_id, &offer.sdp).await;
            
            *session.webrtc.write().await = Some(std::sync::Arc::new(transport));
            return Ok(stream_url);
        }
    }
    
    Err("Failed to create offer".into())
}

#[tauri::command]
async fn switch_monitor(index: u8, session: tauri::State<'_, AppSession>) -> Result<(), String> {
    info!("Switching monitor to {}", index);
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
    client: tauri::State<'_, Option<networking::signaling::SignalingClient>>
) -> Result<(), String> {
    info!("Host connection approval: {}", allowed);
    if !allowed {
        *session.pending_offer.write().await = None;
        session.pending_ice.write().await.clear();
        return Ok(());
    }

    let offer_data = session.pending_offer.write().await.take();
    if let Some((session_id, sdp)) = offer_data {
        let c = client.inner().as_ref().unwrap().clone();
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
        
        let (ice_tx, mut ice_rx) = tokio::sync::mpsc::channel::<webrtc::ice_transport::ice_candidate::RTCIceCandidateInit>(32);
        let (data_tx, mut data_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(128);
        let (video_tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(16);
        
        let c_clone = c.clone();
        let sid = session_id.clone();
        
        tokio::spawn(async move {
            while let Some(candidate) = ice_rx.recv().await {
                c_clone.send_ice_candidate(&sid, &candidate.candidate, candidate.sdp_mid, candidate.sdp_mline_index).await;
            }
        });
        
        if let Ok(transport) = webrtc_lib::transport::WebRtcTransport::new(true, ice_tx, data_tx, video_tx).await {
            let vc = transport.video_channel.clone();

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
                                    }
                                    DataPacket::Disconnect => {
                                        info!("Host received Disconnect packet, closing session.");
                                        let _ = pc.close().await;
                                        break;
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
                                loop {
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

                                    match capture.capture_frame() {
                                        Ok(frame) => {
                                            match video::VideoEncoder::encode(&mut encoder, &frame) {
                                                Ok(encoded) if !encoded.data.is_empty() => {
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
                                            // No new frame within the timeout window; the screen
                                            // is static. Nothing to send, just continue.
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
    tracing_subscriber::fmt::init();
    
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path_resolver().app_data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
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

                        app_handle.manage(Some(client));
                    }
                    Err(e) => {
                        error!("Failed to connect to signaling server: {}", e);
                        app_handle.manage(None::<networking::signaling::SignalingClient>);
                    }
                }
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_devices,
            get_diagnostics,
            connect_to_device,
            switch_monitor,
            send_input_event,
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
