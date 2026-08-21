# Phase 2 — Windows Screen Capture
Implement Phase 2 only.

Implement Windows screen capture.

The host must run as a NORMAL USER application.

Do NOT create:
- Windows service
- kernel driver
- privileged process
- system-wide component

Use appropriate Windows APIs:
- Windows Graphics Capture
- Desktop Duplication API
- Direct3D

Choose the API based on actual suitability for:
- low latency
- high FPS
- GPU-friendly capture
- multiple monitor expansion later

Create:
ScreenCapture trait/interface
WindowsScreenCapture implementation

The capture API should expose:
- frame
- timestamp
- width
- height
- pixel format
- monitor ID

Requirements:
- Primary monitor initially.
- Configurable FPS.
- Resolution detection.
- Display change detection.
- Clean shutdown.
- Capture error handling.
- Avoid unnecessary CPU/GPU copies.

Create a diagnostic executable that reports:
- FPS
- frame time
- resolution
- CPU
- memory
- capture errors

Test on my Windows PC without administrator privileges.

If Windows security/policy prevents a capability, report it clearly rather than trying to bypass it.

STOP after Phase 2.
