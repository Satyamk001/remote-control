# Phase 20 — WebRTC Native Tunneling

Implement Phase 20 only.

Remove the mocked connection logic and integrate the Rust WebRTC state machine.

Target:
Establish a direct P2P tunnel over the Node.js signaling server between the host and the client.

Steps:

1. Introduce WebRTC signaling protocol types (`SdpOffer`, `SdpAnswer`, `IceCandidate`).
2. Establish event broadcasting from the signaling client to the Tauri backend.
3. Handle WebRTC Concurrency & Lifetimes without deadlocking the `RwLock` state.
4. Implement ICE Candidate pipelining via `mpsc` channels.

Produce:

- Rust codebase compiled successfully.
- WebRTC P2P tunnel handshake visible in terminal logs.

Do not sacrifice stability for immediate visual results (video rendering is Phase 23).

STOP after Phase 20.
