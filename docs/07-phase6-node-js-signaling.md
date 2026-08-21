# Phase 6 — Node.js Signaling
Implement Phase 6 only.

Build the Node.js signaling server.

Technology:
Node.js
TypeScript
WebSocket

Do NOT add:
- PostgreSQL
- Redis
- Kafka
- microservices

Responsibilities:
- session creation
- temporary session IDs
- host registration
- client join
- SDP exchange
- ICE candidate exchange
- session lifecycle
- disconnect handling
- session expiration
- message validation

Create:
SessionManager
ConnectionManager
SignalingController
MessageValidator
SessionExpiryManager

Use strict TypeScript types and discriminated unions.

The server must NOT relay video.

Target:
Windows Host
     ↓
Node.js Signaling
     ↓
Ubuntu Client
     ↓
WebRTC P2P

Add:
- unit tests
- WebSocket integration tests
- malformed-message tests
- session expiration tests
- concurrent connection tests

STOP after Phase 6.
