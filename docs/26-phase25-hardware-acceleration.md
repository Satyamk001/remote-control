# Phase 25 — Hardware Acceleration

Implement Phase 25 only.

Achieve maximum latency reduction by shifting H.264 encode/decode workloads directly to GPU silicon, deprecating the WSL `flate2` mock encoder.

Target:
Sub-10ms encode latency for 1080p/60FPS streams.

Steps:

1. Pivot the compiler target back from `x86_64-pc-windows-gnu` (WSL) to the native MSVC toolchain to access Windows SDK libraries.
2. Integrate an FFI-based crate or direct Win32 MF APIs into `video::encoder::HardwareVideoEncoder`.
3. Support Nvidia NVENC / Intel Quick Sync.
4. Establish a Zero-Copy Pipeline, ensuring DXGI captured textures remain on VRAM during the encoding step to avoid expensive CPU/GPU memory swaps.

Produce:

- Native MSVC build capabilities without WSL.
- True GPU-accelerated video encoding and decoding.

STOP after Phase 25.
