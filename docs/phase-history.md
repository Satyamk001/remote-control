# Phase History

Compact record of the phased implementation plan (formerly the per-phase prompt
files `docs/01`–`26`). Each phase was implemented one at a time, built, tested,
and documented. Detailed outcomes live in `PROJECT_STATUS.md`; design rationale
in `decisions.md`; requirements in `00-master-requirements.md`.

| Phase | Title | Goal / Scope (and explicit constraints) |
|------|-------|------------------------------------------|
| 0 | Architecture + Repository | Establish modular monorepo (Cargo workspace + Tauri/React + Node). Define boundaries. Minimal buildable projects only; no full UI; no admin/services. |
| 1 | Rust Networking Foundation | LAN-only UDP/TCP transports, binary framing, seq/timestamps, heartbeat, timeout, lifecycle, logging. `VideoPacket`/`InputEvent` as protocol defs only. No WebRTC/STUN/capture. |
| 2 | Windows Screen Capture | DXGI Desktop Duplication at user level (no driver/service). `ScreenCapture` trait exposing frame/timestamp/size/format/monitor; FPS + display-change detection; diagnostic exe. |
| 3 | LAN Screen Streaming | First end-to-end capture→encode→LAN→decode→render (Windows→Ubuntu). Target 1080p/30. Measure per-stage latency. No WebRTC/input/auth/audio. |
| 5 | WebRTC Media + Data Plane | Replace LAN transport with WebRTC: SDP/ICE, video track, `input-events` DataChannel, stats, encryption. Manual dev signaling. No STUN/TURN yet. |
| 6 | Node.js Signaling | WebSocket control plane: session create, host register, client join, SDP/ICE exchange, lifecycle, expiry, validation. No DB/microservices. Must not relay video. Tested. |
| 7 | Internet P2P | Public-internet test via ICE+STUN (no TURN). Measure RTT/loss/jitter/ICE type; document NAT success/failure. (See decisions.md NAT findings.) |
| 8 | TURN Fallback | `coturn` relay fallback: direct P2P first, else TURN. Authenticated TURN only. Diagnostics distinguish P2P vs relay. |
| 9 | Authentication + Security | Device identity (ed25519), temp session creds, expiry, explicit host approval, authorization, message validation, rate limiting, audit log, permission model. No remote shell. |
| 10 | Clipboard & File Transfer | Chunked CRC32 file transfer with path-traversal protection → `downloads/`; `arboard` clipboard sync. All over WebRTC DataChannel, permission-gated. |
| 11 | Audio & Multiple Monitors | `cpal` WASAPI loopback audio over DataChannel (permission-gated); DXGI multi-monitor enumerate + hot-swap without connection teardown. |
| 12 | Low-Latency & Hardware Encoding | HardwareVideoEncoder (Media Foundation) stub + SoftwareVideoEncoder (deflate) fallback; MetricsTracker; adaptive 60↔30 FPS on encode/RTT thresholds. |
| 13 | Tauri + React + MeetUI Client | Production desktop GUI: MeetUI-styled components, IPC bridges, Dashboard/ActiveSession/Diagnostics views. React = presentation; Rust = engine. |
| 14 | Host Agent UI | Unified host+client in one app. HostAgentPanel: Device ID, incoming-request dialog, active-session banner, forceful disconnect. Non-admin. |
| 15 | Packaging & Distribution | Portable ZIP (no MSI/WiX): WSL cross-compile release exe + WebView2Loader.dll into `release_build/`. Native bundling disabled. |
| 16 | Reliability & Auto-Reconnection | `connection_state_changed` IPC events; blurred "Reconnecting…" overlay; backend monitors `Failed` state for ICE restart. |
| 17 | Security Identity Integration | Wire ed25519 Identity into Tauri lifecycle (persist keypair, manage state); `get_local_device_id` returns real hex public key. |
| 18 | Signaling Integration | Real peer discovery: Node `get_hosts`/`hosts_list`; Rust `SignalingClient` (tokio-tungstenite) registers on launch; `get_devices` serves live peers. |
| 19 | Final Architecture Review | `cargo fix` cleanup; verify layer boundaries; author `final-architecture.md`. |
| 20 | WebRTC Native Tunneling | Remove mocked connect logic; wire UI to the Rust `webrtc` crate; negotiate real `sdp_offer` over signaling. |
| 21 | Interactive Session Approval | Host authorization gate: IPC events drive a secure Accept/Reject prompt on incoming connection. |
| 22 | Data Channel Integration | Unified `DataPacket` binary enum multiplexing keystrokes, mouse, and auxiliary payloads over the DataChannel. |
| 23 | Tauri Video Rendering Pipeline | Internal localhost HTTP loopback server rendering Rust frame buffers into the React view (worked around WebView limits). |
| 24 | MVP Polish | Robust teardown on disconnect; clipboard sync via `arboard`; live telemetry (RTT/FPS/bitrate) into the UI. |
| 25 | Hardware Acceleration | Zero-copy DXGI→VRAM foundation; `windows` crate COM bindings (DXGI/D3D11/Media Foundation) with WSL fallback. |

> Phase 4 was folded into Phase 3/5 and has no standalone document.

## Post-phase hardening (real implementations)

After the 25 numbered phases (which left several pipelines mocked), the mocks
were replaced with working implementations: real `SendInput` injection, real
DXGI readback, real H.264 (OpenH264 → WebCodecs), RTT-adaptive bitrate, wired
audio with format negotiation, file upload, per-stage metrics, configurable
endpoints, and a startup-race fix. See `PROJECT_STATUS.md` for the authoritative
current state and remaining work.
