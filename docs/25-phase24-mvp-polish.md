# Phase 24 — MVP Polish

Implement Phase 24 only.

Add crucial remote desktop usability features like Clipboard Synchronization, Multi-Monitor selection, and live telemetry data visualization.

Target:
Feature-parity with minimum viable product (MVP) requirements.

Steps:

1. Implement a heartbeat check on the host clipboard via `arboard`.
2. Encode clipboard changes as `DataPacket::Clipboard` and transmit across the WebRTC DataChannel.
3. Respect the host's `Permissions` bitflag to block/allow clipboard sync.
4. Update the frontend UI to dispatch the `switch_monitor()` IPC.
5. Replace the mocked `get_diagnostics()` payloads with dynamic WebRTC `get_stats()` polling for RTT, FPS, and Bitrate.

Produce:

- Clipboard sync working bidirectionally.
- Live performance telemetry replacing UI mocks.
- Multi-monitor DXGI support.

STOP after Phase 24.
