# Phase 5 — WebRTC Media + Data Plane
Implement Phase 5 only.

Replace the prototype LAN media/input transport with WebRTC.

Target:
Windows Host
      ↕
WebRTC
      ↕
Ubuntu Client

Use WebRTC for:
- video
- remote input via RTCDataChannel

The Node.js signaling server is NOT implemented yet.

Create a temporary/manual signaling mechanism only for development.

Implement:
- SDP offer
- SDP answer
- ICE candidates
- WebRTC connection state
- video track
- RTCDataChannel
- connection statistics
- encrypted WebRTC transport

Architecture:

ScreenCapture
    ↓
VideoEncoder
    ↓
WebRTC Video Track

InputEvent
    ↓
RTCDataChannel
    ↓
WindowsInputInjector

Do not tightly couple WebRTC to screen capture.

Verify:
- video works
- input works
- connection closes cleanly
- reconnect works
- statistics are available

Measure:
## FPS
bitrate
## RTT
packet loss
jitter
end-to-end latency

Do not implement STUN/TURN yet.

STOP after Phase 5.
