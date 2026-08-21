# Phase 5: WebRTC Media & Data Plane

## Goal
Replace the prototype TCP/UDP LAN transport with WebRTC to prepare for NAT traversal, Internet streaming, and robust congestion control.

## Decisions Made
1. **WebRTC Stack**: Used the `webrtc` pure-Rust crate (v0.11) instead of C++ libwebrtc bindings for easier cross-compilation and memory safety.
2. **Video Streaming**: Initialized `TrackLocalStaticSample` bound to an H264 RTP transceiver to push raw/encoded video frames.
3. **Data Channel**: Established an `RTCDataChannel` named `input-events` to securely pipe remote input commands (mouse/keyboard).
4. **Input Abstraction**: Created `crates/input` defining discriminated unions for InputEvents and a `WindowsInputInjector` trait.

## Status
Scaffolded successfully. Code compilation is currently blocked due to missing Microsoft Visual C++ Build Tools on the host machine.
