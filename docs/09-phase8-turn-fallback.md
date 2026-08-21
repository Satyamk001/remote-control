# Phase 8 — TURN Fallback
Implement Phase 8 only.

Add TURN fallback using coturn.

Connection strategy:
1. Attempt direct P2P.
2. If unavailable, use TURN relay.

Architecture:

Client
    │
    ├── Direct P2P
    │
    └── TURN fallback
             │
             ▼
          Host

Requirements:
- STUN configuration
- TURN configuration
- ICE server priorities
- authenticated TURN
- short-lived credentials where practical
- connection-type detection
- diagnostics showing P2P vs TURN
- forced TURN testing
- direct P2P testing

Do not expose an unauthenticated/open TURN server.

Keep coturn separate from the Node.js signaling server.

Document deployment requirements.

Measure:
P2P latency
TURN latency
P2P bandwidth
TURN bandwidth
CPU/memory impact

STOP after Phase 8.
