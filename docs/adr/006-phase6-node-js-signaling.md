# Phase 6: Node.js Signaling

## Goal
Build a lightweight, stateless signaling server to negotiate WebRTC SDP and ICE exchanges between the Windows Host and the remote Viewer over the internet.

## Decisions Made
1. **Technology**: Selected Node.js, TypeScript, and the `ws` library.
2. **Statelessness**: Avoided databases (Redis/Postgres) entirely. Active sessions are stored in an in-memory `Map` since WebRTC negotiation is ephemeral.
3. **Protocol**: Designed a strict JSON protocol using Discriminated Unions (`register_host`, `join_session`, `sdp_offer`, etc.) to prevent malformed data.
4. **Lifecycle**: Implemented automated background sweeping (`SessionExpiryManager`) to purge abandoned sessions.

## Status
Implemented, fully unit-tested with Jest (100% pass rate), and verified manually.
