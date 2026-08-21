# Phase 11: Audio & Multiple Monitors

## Goal
Implement system loopback audio capture and stream it via WebRTC DataChannels. Allow the client to seamlessly detect and switch between multiple physical monitors attached to the host.

## Decisions Made
1. **Audio Capture**:
   Utilized the cross-platform `cpal` crate with the WASAPI loopback interface on Windows to capture system audio. The raw PCM `f32` data is passed through a lock-free `ringbuf` from the `cpal` audio thread into a background Tokio task in the streaming pipeline, which transmits it as `AudioPacket` payloads over the WebRTC DataChannel.
2. **Audio Permissions**:
   The audio capture loop strictly checks the `permissions.audio` flag. If the host denies audio capture during the initial connection prompt, the `cpal` stream is never initialized, ensuring total privacy.
3. **Multiple Monitors (DXGI)**:
   Enhanced `WindowsScreenCapture` to accept an `output_index` (Monitor ID) and implemented a `list_monitors()` function using Direct3D 11's `EnumOutputs`. 
4. **Hot-Swapping Monitors**:
   When the host receives a `SwitchMonitor` message, it updates an `AtomicU32` flag. The main video encoding loop detects the change and dynamically hot-swaps the underlying Desktop Duplication API interface (`IDXGIOutputDuplication`) without requiring a complete teardown of the WebRTC connection or video encoder, enabling instantaneous switching.

## Status
Completed. Rust codebase compiled successfully with `cpal` and multiple monitor support.
