# Phase 12 — Low-Latency + Hardware Encoding
Implement Phase 12 only.

Optimize the application for low-latency remote desktop.

Primary target:
1080p @ 60 FPS

Secondary:
1440p @ 60 FPS
4K @ 60 FPS where hardware/network conditions permit.

Implement hardware video encoding/decoding.

Windows targets:
## Nvidia NVENC
Intel Quick Sync
AMD hardware encoding where practical

Create:
HardwareVideoEncoder
SoftwareVideoEncoder

Hardware encoder should be preferred when available.

Software fallback must remain available.

Optimize:
- GPU capture
- GPU encoding
- memory copies
- frame buffering
- packetization
- decode
- rendering

Implement:
- adaptive bitrate
- adaptive FPS
- frame dropping
- jitter buffering
- keyframe management
- congestion handling
- bandwidth estimation

Measure before optimizing.

Expose:
capture latency
encode latency
network latency
decode latency
render latency
end-to-end latency
## FPS
bitrate
packet loss
jitter
## RTT
## CPU
## GPU
memory

Create a performance test harness.

Do not make performance claims without measurements.

Compare:
Software encoding
vs
Hardware encoding

Document the measured improvement.

STOP after Phase 12.
