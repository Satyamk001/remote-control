# Phase 1 — Rust Networking Foundation
Implement Phase 1 only.

Build the low-level networking foundation.

This phase is LAN-only.

Test environment:
Windows PC <──── LAN ────> Ubuntu PC

Do not implement:
- Screen capture
- WebRTC
- STUN
- TURN
- Internet connectivity
- Remote input
- React UI beyond minimal diagnostics

Implement:
- UDP transport
- TCP transport where useful
- Transport abstraction
- Binary message serialization
- Message framing
- Sequence numbers
- Timestamps
- Heartbeat
- Timeout detection
- Connection lifecycle
- Graceful shutdown
- Structured logging
- Error handling

Create typed protocol messages:
Heartbeat
Ping
Pong
ConnectionRequest
ConnectionResponse
VideoPacket
InputEvent

VideoPacket and InputEvent should only be protocol definitions for now.

Create:
Transport trait/interface
UdpTransport
TcpTransport

Test:
Windows → Ubuntu
Ubuntu → Windows

Verify:
- connection
- packet transfer
- malformed packet handling
- heartbeat
- timeout
- disconnect
- reconnect

The implementation must work without administrator privileges.

Add unit and integration tests.

STOP after Phase 1.
