# Phase 3: LAN Screen Streaming

## Goal
Integrate the networking foundation and the video capture logic to create an end-to-end streaming pipeline suitable for local area networks.

## Decisions Made
1. **Encoding**: Defined `VideoEncoder` and `VideoDecoder` traits. (Implementation targets H264 hardware encoding where possible, though WebRTC will eventually handle this).
2. **Pipeline**: Created the `remote-core` streaming logic (`streaming.rs`) that continuously polls the `ScreenCapture` trait, encodes the frame, and pushes it to the networking transport layer.
3. **Concurrency**: Used `tokio::spawn` loops and channels for decoupled frame capturing and network transmission to avoid blocking the main thread.

## Status
Implemented successfully in `remote-core`.
