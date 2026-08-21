# Phase 22 — Data Channel Integration

Implement Phase 22 only.

Connect the decoupled `input`, `file-transfer`, and `audio` crates to the active WebRTC `RTCDataChannel`.

Target:
Multiplexed binary payloads traversing the WebRTC tunnel triggering actual OS-level hooks.

Steps:

1. Instantiate a single `RTCDataChannel` named `events`.
2. Define a unified `DataPacket` binary enum using `bincode`.
3. Pipe client keystrokes and mouse movements down the channel.
4. Intercept `DataPacket::Input` on the host side and push to the `input` crate.

Produce:

- Functional remote control (keyboard/mouse).
- Audio streaming fallback/tests.

STOP after Phase 22.
