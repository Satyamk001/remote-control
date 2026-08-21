# Phase 2: Windows Screen Capture

## Goal
Implement a high-performance Windows screen capture mechanism without requiring administrator privileges.

## Decisions Made
1. **API Choice**: Selected Desktop Duplication API (DXGI) via the `windows` crate, as it provides hardware-accelerated frame capturing at the user level without drivers.
2. **Abstraction**: Created a `ScreenCapture` trait in `crates/video` to allow for cross-platform swapping in the future, even though this phase focused on Windows.
3. **Frame Handling**: Defined a `VideoFrame` structure representing raw RGBA byte arrays returned from the DXGI surface.

## Status
Implemented successfully in `crates/video/src/capture.rs`.
