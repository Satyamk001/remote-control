# ADR 018: Phase 18 - Signaling Server Integration

## Status
Accepted

## Context
Up until Phase 17, the desktop UI was driven entirely by mocked data. Although the UI successfully displayed the local Device ID dynamically, the "Dashboard" devices list and "Connect" buttons did not bridge to the remote internet. We needed to integrate the Rust engine with our Node.js signaling server to discover other online peers and facilitate WebRTC connection negotiation.

## Decision
We implemented a real-time `SignalingClient` in Rust and upgraded the Node.js signaling server:
1. **Node.js Upgrades**: We expanded the WebSocket protocol defined in Phase 6 to support a `get_hosts` request and a `hosts_list` response. The server now tracks `deviceId` directly in its Session memory alongside the generated UUID `sessionId`.
2. **Rust SignalingClient (`tokio-tungstenite`)**: We built a robust background WebSocket loop in the `networking` crate. When the native Tauri application launches, it automatically establishes a persistent `ws://` connection to the signaling server and registers its identity.
3. **Tauri IPC Integration**: The `get_devices` Tauri IPC command was rewritten to pull live, active peer data from the Rust `SignalingClient` memory pool rather than returning hardcoded mock JSON arrays.

## Consequences
**Positive:**
- The Dashboard now dynamically lists actual devices running the application instead of static mock files.
- The architectural bridge between Tauri (UI) -> Rust (Engine) -> Node.js (Signaling) is fully functional.

**Negative:**
- The signaling connection is currently hardcoded to `ws://127.0.0.1:8081`. Future iterations will need to accept environment variables or configuration files to connect to a public cloud instance.
