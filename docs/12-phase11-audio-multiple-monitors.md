# Phase 11 — Audio + Multiple Monitors
Implement Phase 11 only.

Add:
1. Remote system audio.
2. Multiple monitor support.

Audio:

Host
 ↓
System Audio Capture
 ↓
Audio Encoder
 ↓
WebRTC
 ↓
Client
 ↓
Audio Output

Support:
- enable/disable
- mute
- volume
- low latency
- A/V synchronization

Multiple monitors:

Detect:
Monitor 1
Monitor 2
Monitor 3

Allow:
- selecting monitor
- switching monitors
- monitor resolution changes
- monitor connect/disconnect

Do not break the existing single-monitor implementation.

Keep monitor capture platform-specific and isolated.

STOP after Phase 11.
