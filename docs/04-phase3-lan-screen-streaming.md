# Phase 3 — LAN Screen Streaming
Implement Phase 3 only.

Build the first complete remote screen streaming prototype.

Architecture:

Windows host
    ↓
Screen Capture
    ↓
Video Encoder
    ↓
LAN Transport
    ↓
Ubuntu client
    ↓
Video Decoder
    ↓
Renderer

Goal:
I must be able to see my Windows desktop from my Ubuntu PC over the LAN.

Start with correctness and reliability.

Do not implement:
- WebRTC
- Internet
- STUN
- TURN
- Remote keyboard
- Remote mouse
- Authentication
- File transfer
- Audio

Create abstractions:
ScreenCapture
VideoEncoder
VideoPacketizer
VideoDecoder
VideoDepacketizer
Renderer
Transport

Include:
- frame IDs
- timestamps
- sequence numbers
- packet validation
- dropped-frame detection
- basic jitter handling

Display diagnostics:
- FPS
- bitrate
- packet loss
- latency
- resolution

Target:
1080p / 30 FPS

Test on:
Windows → Ubuntu

Measure:
- capture latency
- encode latency
- network latency
- decode latency
- render latency
- end-to-end latency

Do not optimize blindly.

STOP after Phase 3.
