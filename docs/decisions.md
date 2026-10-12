# Architecture Decision Log

Consolidated record of the architectural decisions made across the project
(formerly `docs/adr/001-019` plus the Phase 7/8 result notes). Each entry is a
decision + rationale; implementation status reflects the state at the time the
decision was recorded. For the current, authoritative state of the codebase see
`PROJECT_STATUS.md` and `final-architecture.md`.

---

## ADR 001 — Repository Architecture (Phase 0) · Accepted
Modular Cargo workspace for Rust crates + separate `apps/desktop` (Tauri/React)
and `apps/signaling-server` (Node.js). Crates: `remote-core`, `screen-capture`,
`video`, `input`, `audio`, `file-transfer`, `networking`, `webrtc`, `security`,
`platform-windows`. UI talks to Rust only via Tauri IPC; signaling is a control
plane that never relays media; no UAC/admin required.

## ADR 002 — Rust Networking Foundation (Phase 1)
`tokio` async runtime; `bincode`+`serde` binary serialization; a generic
transport trait abstracting TCP/UDP (and later WebRTC). Lives in
`crates/networking`.

## ADR 003 — Windows Screen Capture (Phase 2)
Desktop Duplication API (DXGI) via the `windows` crate — hardware-accelerated,
user-level, no driver. `ScreenCapture` trait for future cross-platform swap;
frames are raw BGRA/RGBA byte arrays with timestamp/size/format/monitor id.

## ADR 004 — LAN Screen Streaming (Phase 3)
`VideoEncoder`/`VideoDecoder` traits; `remote-core` streaming loop polls
capture → encode → transport. Decoupled capture/transmit via `tokio` tasks +
channels.

## ADR 005 — WebRTC Media & Data Plane (Phase 5)
Pure-Rust `webrtc` crate (v0.11) over C++ libwebrtc (easier cross-compile,
memory-safe). Video via H264 track originally; `input-events` RTCDataChannel for
remote input; `crates/input` defines `InputEvent` unions + `WindowsInputInjector`.

## ADR 006 — Node.js Signaling (Phase 6)
Node.js + TypeScript + `ws`. Stateless (in-memory `Map`, no DB). Strict JSON
discriminated-union protocol (`register_host`, `join_session`, `sdp_offer`…).
Background `SessionExpiryManager` sweeps abandoned sessions. Jest-tested.

## ADR 007 — Internet P2P + STUN (Phase 7)
Public Google STUN servers in `RTCIceServer`; ICE candidate logging + connection
state hooks. See "NAT traversal findings" below.

## ADR 008 — TURN Fallback (Phase 8)
`coturn` via `docker-compose.turn.yml` with long-term credentials (no open/
unauthenticated TURN). `on_ice_candidate` distinguishes `typ relay` vs `typ
srflx`. Signaling server stays unaware of TURN; creds injected into Rust ICE
config. (Later made env-configurable; TURN now defaults off — see
`docs/configuration.md`.)

## ADR 009 — Authentication & Security (Phase 9)
`Permissions` bitflag (Screen/Mouse/Keyboard/Clipboard/Files/Audio/Terminal);
explicit host approval prompt; `ed25519-dalek` persistent device identity;
IP token-bucket rate limiting + `[AUDIT]` logging on the signaling server;
Terminal/File access denied by default; no unrestricted remote shell.

## ADR 010 — Clipboard & File Transfer (Phase 10)
`FileReceiver` state machine with CRC32 per-chunk verification + sequential
ordering; strict filename sanitization (rejects `/`, `\`, `..`) → writes only to
`downloads/`. `arboard` for clipboard. All data multiplexed over the one WebRTC
peer connection. Host permission gating drops disallowed file/clipboard data.

## ADR 011 — Audio & Multiple Monitors (Phase 11)
`cpal` WASAPI loopback → lock-free `ringbuf` → Tokio task → `AudioPacket` over
DataChannel; gated by `permissions.audio`. DXGI `output_index` + `list_monitors()`
(D3D11 `EnumOutputs`); `SwitchMonitor` hot-swaps `IDXGIOutputDuplication` via an
`AtomicU32` without tearing down WebRTC/encoder.

## ADR 012 — Low-Latency & Encoding (Phase 12)
`HardwareVideoEncoder` (Media Foundation NVENC/QuickSync/AMF) as a structural
stub due to WSL cross-compile linking limits; `SoftwareVideoEncoder` via
`flate2` deflate as runnable fallback. `MetricsTracker` profiles capture/encode/
RTT/drops. Adaptive control: >15ms encode or >100ms RTT downshifts 60→30 FPS.

## ADR 013 — Tauri + React + MeetUI Client (Phase 13)
MeetUI design language re-implemented locally (Tailwind + `lucide-react`,
`tailwind-merge`/`clsx`) since the sibling repo was inaccessible. IPC bridge for
`get_devices`/`connect_to_device`/`get_diagnostics`/`switch_monitor`. Views:
Dashboard, ActiveSession, DiagnosticsPanel. Monolithic binary via WSL GNU target.

## ADR 014 — Host Agent UI (Phase 14)
Unified host+client into one Tauri app (not a separate exe). `HostAgentPanel`
shows Device ID, incoming-request dialog, active-session banner, forceful
disconnect. IPC: `get_local_device_id`, `approve_connection`,
`get_host_session_status`, `terminate_host_session`.

## ADR 015 — Packaging (Phase 15) · Accepted
Portable ZIP instead of MSI/NSIS (no MSVC/WiX on host). Tauri native bundling
disabled (`"targets": []`); `package_release.sh` cross-compiles the release exe
in WSL and assembles `desktop.exe` + `WebView2Loader.dll` into `release_build/`.
Trade-off: no installer/updater/Start-Menu integration.

## ADR 016 — Reliability & Auto-Reconnection (Phase 16) · Accepted
Rust emits `connection_state_changed` IPC events; React renders a blurred
"Reconnecting…" overlay on `reconnecting`; backend monitors
`RTCPeerConnectionState::Failed` for ICE restart. State logic (Rust) separated
from presentation (React).

## ADR 017 — Security Identity Integration (Phase 17) · Accepted
Wired `remote_core::security::Identity` into the Tauri lifecycle: resolve app
data dir in `setup`, `Identity::load_or_generate` persists the `ed25519`
keypair, injected via `app.manage()`; `get_local_device_id` returns the
hex-encoded public key (replacing the earlier mock).

## ADR 018 — Signaling Integration (Phase 18) · Accepted
Node server extended with `get_hosts`/`hosts_list` (tracks deviceId + sessionId).
Rust `SignalingClient` (`tokio-tungstenite`) connects + registers on launch;
`get_devices` serves live peers. (Endpoint later made configurable via
`REMOTE_CONTROL_SIGNALING_URL`; default is the deployed Render instance.)

## ADR 019 — Final Architecture Review (Phase 19) · Accepted
`cargo fix` dead-code cleanup; verified layer boundaries (Presentation/
Application/Infrastructure/Platform); authored `final-architecture.md`.

---

## NAT Traversal Findings (Phase 7/8 testing notes)

**STUN config** (`crates/webrtc/src/transport.rs`): Google STUN discovers public
IP/port mappings. Outcomes depend on NAT type:

- **Full/Restricted Cone NAT** → direct P2P succeeds via `srflx` candidates.
  Expected: RTT < 50ms, jitter < 10ms, ~5–10 Mbps.
- **Symmetric NAT (both sides)** → hole punching fails (external port is not
  predictable); requires a **TURN relay**. Without TURN the peer state goes
  `Failed`.

**Forced-TURN test**: `docker-compose -f docker-compose.turn.yml up -d`, set
`ice_transport_policy = Relay` in the WebRTC config, connect, and confirm the
`---> DISCOVERED TURN RELAY CANDIDATE <---` log. Expected: P2P ~10–50ms vs TURN
~50–150ms+ (relayed), minor extra host CPU for relay heartbeat.

> Historical build blocker (now resolved via WSL GNU cross-compile): the host
> lacked MSVC `link.exe`, so earlier phases could not produce native binaries on
> Windows directly.
