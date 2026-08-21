# Final Architecture Review

This document serves as the official architecture specification for the Remote Control application, addressing the final state of the system across the Presentation, Application, Domain, and Infrastructure layers.

## 1. System Architecture

The application follows a distributed P2P (Peer-to-Peer) topology, orchestrated by a central WebSocket Signaling Server.

- **Tauri Desktop Client**: A hybrid application using a React/TypeScript frontend (Presentation) bound to a native Rust backend (Application/Domain).
- **Signaling Server**: A Node.js/WebSocket server acting as the rendezvous point for IP discovery and WebRTC SDP (Session Description Protocol) exchange.
- **P2P Tunnel**: Direct WebRTC connections established between Host and Viewer clients for low-latency media and data transport.

## 2. Component Architecture

- `desktop` (App/Tauri): The main entry point. Houses the React frontend and handles IPC routing to the core engine.
- `remote-core` (App Layer): Orchestrates session lifecycles, permissions, identity, and threads.
- `webrtc` / `networking` (Infrastructure): Wraps `webrtc-rs` and `tokio-tungstenite` to handle ICE traversal, DTLS/SRTP encryption, and WebSocket signaling.
- `screen-capture` / `audio` / `input` (Platform): Interfaces with OS-specific APIs (DXGI for Windows capture, CPAL for audio loopback, hook APIs for input injection).
- `video` (Domain): Manages H.264 encode/decode pipelines, interfacing with both software and mock-hardware encoders.

## 3. Data Flow

1. **Frontend (React)** requests data via Tauri `invoke()`.
2. **Backend (Rust)** handles the IPC command, often interacting with `remote-core` or `networking` background tokio tasks.
3. State is passed back to the frontend either synchronously via Return values or asynchronously via `app_handle.emit_all()` (e.g., `connection_state_changed`).

## 4. Connection Flow

1. Host and Viewer launch the native `.exe`.
2. `tokio-tungstenite` establishes a WebSocket connection to `ws://127.0.0.1:8081`.
3. Host sends `register_host` with its `DeviceId`.
4. Viewer sends `get_hosts` to discover the Host, then initiates a `join_session` request.
5. The Signaling Server proxies `sdp_offer`, `sdp_answer`, and `ice_candidate` messages between the two peers.
6. A WebRTC `RTCPeerConnection` is successfully established natively.

## 5. Authentication Flow

- Centralized accounts/passwords are NOT used.
- The `security` crate leverages `ed25519-dalek` to generate a local CSPRNG keypair (`identity.key`) on first launch.
- The Public Key (VerifyingKey) serves as the globally unique, cryptographically verifiable `DeviceId`.
- When a Viewer attempts to connect, the Host's `remote-core` triggers a `[y/N]` interactive prompt (or UI dialog) to authorize the session.

## 6. Video Pipeline

- **Capture**: `screen-capture` enumerates monitors using DXGI and grabs RGBA frames.
- **Encode**: `video` compresses frames into H.264 NALUs using an Adaptive Bitrate controller driven by network RTT.
- **Transport**: `webrtc` packetizes the NALUs into RTP packets and streams them over the SRTP UDP socket.

## 7. Input Pipeline

- **Transport**: Keyboard/Mouse events are JSON-serialized and sent over the WebRTC `RTCDataChannel` (`input-events`).
- **Authorization**: The Host inspects the session `Permissions` bitflag. If Input is allowed, the event is passed to the `input` crate.
- **Injection**: The `input` crate translates the cross-platform event back into OS-specific Win32 hooks to move the mouse or press keys.

## 8. File Transfer Pipeline

- Files are dragged/dropped into the UI.
- The payload is chunked (e.g., 16KB blocks), CRC32-hashed, and sent over a dedicated WebRTC DataChannel.
- The receiving `file-transfer` crate reassembles the file, verifies the hash, and strictly sanitizes the filename to prevent path traversal attacks before saving to the `downloads` directory.

## 9. Failure Handling

- Network drops trigger a `connection_state_changed` IPC event.
- The UI gracefully renders a blurred "Reconnecting..." overlay (`BouncingLoader`) instead of abruptly crashing.
- `remote-core` monitors the `RTCPeerConnectionState`. If it transitions to `Failed`, an ICE Restart is triggered via the signaling server to re-establish NAT traversal.

## 10. Security Model

- **Signaling**: WebSocket endpoints are protected by IP rate-limiting (`MAX_TOKENS = 100`).
- **Transport**: All P2P media and data channels are strictly encrypted using DTLS 1.2 and SRTP (Standard WebRTC).
- **Identity**: Ed25519 prevents Device ID spoofing.
- **Authorization**: The Host explicitly governs granular `Permissions` (Screen Only, Input, File Transfer).

## 11. Deployment Architecture

- **Host Machine**: Due to MSVC/WiX constraints on standard Windows environments without Visual Studio, the Rust compiler targets `x86_64-pc-windows-gnu` via a WSL (Windows Subsystem for Linux) cross-compilation pipeline.
- **Release**: A portable ZIP archive is generated containing the `Remote Control.exe` and `WebView2Loader.dll`.
- **Signaling Server**: Deployed as a standard Node.js Docker container.
- **TURN Server**: `coturn` is deployed via `docker-compose.turn.yml` to relay traffic when direct P2P fails due to symmetric NATs.

## 12. Known Limitations

- The `connect_to_device` Tauri command is currently mocked in the Rust backend. While the signaling client and cryptographic identity are fully integrated, the *trigger* that links the UI button click to the underlying `WebRtcTransport::new()` state machine is pending implementation.
- Hardware video encoding is mocked on Linux/WSL environments.
- Support is currently tightly coupled to Windows (DXGI/Win32) despite the modular trait abstractions.

## 13. Future Improvements

- **Phase 20**: Complete the final integration of the `connect_to_device` IPC command to trigger the WebRTC ICE negotiation natively.
- **Cross-Platform**: Implement the MacOS (AVFoundation) and Linux (X11/Wayland) screen capture backends.
- **Signaling**: Migrate hardcoded `127.0.0.1` IPs to configurable environment variables or a settings UI for cloud deployments.
