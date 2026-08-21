# Phase 23 — Tauri Video Rendering Pipeline

Implement Phase 23 only.

Solve the Tauri/WebView constraint by extracting decoded WebRTC H.264 frames from the Rust backend and pushing them into a React UI component (`<canvas>` or `<video>`) at 60 FPS.

Target:
Client successfully renders the host's desktop screen at 60 FPS natively in the UI.

Steps:

1. Instantiate a lightweight local HTTP loopback server (e.g., `127.0.0.1:4000`) inside the Tauri Rust process.
2. Hook the `on_track` listener in `WebRtcTransport` to intercept RTP video packets.
3. Pipe packets through the `video` decoder crate.
4. Push uncompressed frames to the local HTTP server rendering buffer.
5. React frontend connects via an `<img src="http://127.0.0.1:4000/stream" />` or Canvas WebSocket buffer.

Produce:

- Real-time video rendering inside the MeetUI client.
- End-to-end screen streaming pipeline.

STOP after Phase 23.
