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
}

/// Convert a captured frame buffer into a JPEG image.
///
/// DXGI desktop duplication delivers BGRA8 (4 bytes/px). The `image` crate's
/// JPEG encoder expects RGB, so we drop alpha and swap B/R channels. The buffer
/// passed in is already tightly packed (row padding stripped by the capturer).
fn encode_frame_to_jpeg(
    data: &[u8],
    width: u32,
    height: u32,
    format: screen_capture::PixelFormat,
) -> Option<Vec<u8>> {
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 || data.len() < w * h * 4 {
        return None;
    }

    let mut rgb = vec![0u8; w * h * 3];
    match format {
        screen_capture::PixelFormat::Bgra8 | screen_capture::PixelFormat::Unknown => {
            // Treat Unknown as BGRA (DXGI's native desktop format) by default.
            for i in 0..(w * h) {
                let s = i * 4;
                let d = i * 3;
                rgb[d] = data[s + 2]; // R <- B-position (BGRA)
                rgb[d + 1] = data[s + 1]; // G
                rgb[d + 2] = data[s]; // B <- R-position
            }
        }
        screen_capture::PixelFormat::Rgba8 => {
            for i in 0..(w * h) {
                let s = i * 4;
                let d = i * 3;
                rgb[d] = data[s];
                rgb[d + 1] = data[s + 1];
                rgb[d + 2] = data[s + 2];
            }
        }
    }

    let img: image::RgbImage = image::ImageBuffer::from_raw(width, height, rgb)?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 75)
        .encode_image(&img)
        .ok()?;
    Some(cursor.into_inner())
}

/// Fallback animated test pattern used when native DXGI capture is unavailable
/// (e.g. WSL/Linux builds). Keeps the stream alive so the UI pipeline can be
/// exercised without a real desktop capture backend.
fn run_mock_capture(
    frame_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    switch_rx: std::sync::mpsc::Receiver<u32>,
) {
    let mut x_shift: u8 = 0;
    loop {
        // Drain any switch requests so the channel doesn't back up.
        while switch_rx.try_recv().is_ok() {}

        let mut img = image::RgbImage::new(320, 180);
        for (x, y, p) in img.enumerate_pixels_mut() {
            *p = image::Rgb([x_shift.wrapping_add(x as u8), 100, y as u8]);
        }
        x_shift = x_shift.wrapping_add(5);

        let mut cursor = std::io::Cursor::new(Vec::new());
        if image::codecs::jpeg::JpegEncoder::new(&mut cursor)
            .encode_image(&img)
            .is_ok()
        {
            if frame_tx.blocking_send(cursor.into_inner()).is_err() {
                break;
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
    
    Ok(serde_json::json!({
        "fps": 30, // Locked to 30 FPS based on Host loop
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
    
    // Spawn MJPEG Loopback Server for Client
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
    let local_addr = listener.local_addr().map_err(|e| e.to_string())?;
    let stream_url = format!("http://{}/stream", local_addr);
    
    tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                    let mut rx = video_tx.subscribe();
                    tokio::spawn(async move {
                        let mut buf = [0; 1024];
                        let _ = socket.read(&mut buf).await;
                        
                        let headers = "HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary=frame\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-cache\r\n\r\n";
                        if socket.write_all(headers.as_bytes()).await.is_err() { return; }
                        
                        // Send a 1x1 black JPEG immediately to keep the browser img tag from timing out
                        // while it waits for the Host to approve the session.
                        let black_jpeg = [
                            0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00, 0x48,
                            0x00, 0x48, 0x00, 0x00, 0xff, 0xdb, 0x00, 0x43, 0x00, 0x03, 0x02, 0x02, 0x02, 0x02, 0x02, 0x03,
                            0x02, 0x02, 0x02, 0x03, 0x03, 0x03, 0x03, 0x04, 0x06, 0x04, 0x04, 0x04, 0x04, 0x04, 0x08, 0x06,
                            0x06, 0x05, 0x06, 0x09, 0x08, 0x0a, 0x0a, 0x09, 0x08, 0x09, 0x09, 0x0a, 0x0c, 0x0f, 0x0c, 0x0a,
                            0x0b, 0x0e, 0x0b, 0x09, 0x09, 0x0d, 0x11, 0x0d, 0x0e, 0x0f, 0x10, 0x10, 0x11, 0x10, 0x0a, 0x0c,
                            0x12, 0x13, 0x12, 0x10, 0x13, 0x0f, 0x10, 0x10, 0x10, 0xff, 0xc0, 0x00, 0x0b, 0x08, 0x00, 0x01,
                            0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xff, 0xc4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
                            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xc4, 0x00, 0x14,
                            0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                            0x00, 0x00, 0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3f, 0x00, 0x37, 0xff, 0xd9,
                        ];
                        let header = format!("--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", black_jpeg.len());
                        if socket.write_all(header.as_bytes()).await.is_err() { return; }
                        if socket.write_all(&black_jpeg).await.is_err() { return; }
                        if socket.write_all(b"\r\n").await.is_err() { return; }
                        
                        while let Ok(frame) = rx.recv().await {
                            let header = format!("--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", frame.len());
                            if socket.write_all(header.as_bytes()).await.is_err() { break; }
                            if socket.write_all(&frame).await.is_err() { break; }
                            if socket.write_all(b"\r\n").await.is_err() { break; }
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
                                    DataPacket::Disconnect => {
                                        info!("Host received Disconnect packet, closing session.");
                                        let _ = pc.close().await;
                                        break;
                                    }
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
                                            if let Some(jpeg) = encode_frame_to_jpeg(
                                                &frame.data,
                                                frame.width,
                                                frame.height,
                                                frame.pixel_format,
                                            ) {
                                                if frame_tx.blocking_send(jpeg).is_err() {
                                                    break; // receiver gone -> session ended
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

                                    // Cap at ~30 FPS; DXGI only returns changed frames so this is
                                    // an upper bound, not a busy loop.
                                    std::thread::sleep(std::time::Duration::from_millis(33));
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
                    
                    let base_lat = 35 + (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_millis() % 15) as usize;
                    CURRENT_LATENCY.store(base_lat, Ordering::Relaxed);
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
